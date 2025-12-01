use dsl::ir::FutureLabel;
use generated::maroon_assembler::{SelectArm, State};
use slab::Slab;

use crate::fiber::FutureId;
use std::collections::HashMap;

pub struct WaitRegistry {
  /// For each source key, a waiter list
  per_key: HashMap<WaitKey, WaitList>,
  /// Storage for intrusive list nodes
  nodes: Slab<WaitNode>,
  /// Active select registrations keyed by id
  regs: Slab<SelectReg>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WaitKey {
  // TODO: should replace strings with some u64s
  Queue(String),
  Future(FutureId),
}

type WaitNodeId = usize;
type SelectRegId = usize;

#[derive(Clone, Debug)]
struct WaitNode {
  prev: Option<WaitNodeId>,
  next: Option<WaitNodeId>,
  reg_id: SelectRegId,
  fiber_id: u64,
}

#[derive(Clone, Debug, Default)]
struct WaitList {
  head: Option<WaitNodeId>,
  tail: Option<WaitNodeId>,
}

#[derive(Clone, Debug)]
enum ArmKind {
  Queue,
  Future,
}

#[derive(Clone, Debug)]
struct ArmResume {
  /// Name of the variable to bind the arriving value to (if any)
  bind: Option<String>,
  /// Next state to push when resuming
  next: State,
}

#[derive(Clone, Debug)]
struct ArmHandle {
  key: WaitKey,
  node_id: WaitNodeId,
  kind: ArmKind,
  resume: ArmResume,
}

#[derive(Clone, Debug, Default)]
struct SelectReg {
  /// Awaiting fiber identity
  fiber_id: u64,
  /// All arms registered for this select
  arms: Vec<ArmHandle>,
}

/// keeps an information that runtime is needed to wake up and run the Fiber
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WakeOutcome {
  pub fiber_id: u64,
  /// to which variable bind the result
  pub bind: Option<String>,
  pub next: State,
}

/// Uniquily identifies in-flight select registration inside WaitRegistry
/// will be useful for timers(maybe not only) because future/queues are "identifiers" itself
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RegisteredSelectId(pub usize);

impl WaitRegistry {
  pub fn default() -> WaitRegistry {
    // No particular reason for 1024 const
    // Maybe later it will be changed, maybe not
    WaitRegistry { per_key: HashMap::default(), nodes: Slab::with_capacity(1024), regs: Slab::with_capacity(1024) }
  }

  pub fn register_select(
    &mut self,
    fiber_id: u64,
    arms: Vec<SelectArm>,
  ) -> RegisteredSelectId {
    // Allocate a registration slot in Slab (empty arms for now)
    // O(1)
    let reg_id = self.regs.insert(SelectReg { fiber_id, arms: Vec::with_capacity(arms.len()) });

    let mut arm_handles: Vec<ArmHandle> = Vec::with_capacity(arms.len());

    for arm in arms.into_iter() {
      match arm {
        SelectArm::Queue { queue_name, bind, next } => {
          let key = WaitKey::Queue(queue_name);
          let node_id = self.nodes.insert(WaitNode { prev: None, next: None, reg_id, fiber_id });
          self.list_push_back(&key, node_id);
          arm_handles.push(ArmHandle {
            key,
            node_id,
            kind: ArmKind::Queue,
            resume: ArmResume { bind: Some(bind), next },
          });
        }
        SelectArm::Future { future_id, bind, next } => {
          // TODO: registry should be responsible for creating ids. I'll change it later
          let fid = FutureId::from_label(future_id, fiber_id);
          let key = WaitKey::Future(fid);
          let node_id = self.nodes.insert(WaitNode { prev: None, next: None, reg_id, fiber_id });
          self.list_push_back(&key, node_id);
          arm_handles.push(ArmHandle { key, node_id, kind: ArmKind::Future, resume: ArmResume { bind, next } });
        }
      }
    }

    // Fill arms into the registration entry
    if let Some(reg) = self.regs.get_mut(reg_id) {
      reg.arms = arm_handles;
    }

    RegisteredSelectId(reg_id)
  }

  /// - returns the first waiter for the key(if there is), unlinks its sibling arms, and removes the registration
  /// - O(selected_arms) ~ O(1)
  pub fn wake_one(
    &mut self,
    key: &WaitKey,
  ) -> Option<WakeOutcome> {
    // Peek head node for this key
    let head_id = {
      let wl = self.per_key.get(key)?;
      wl.head?
    };

    // Read node's registration and fiber
    let (reg_id, fiber_id) = {
      // TODO: should I panic here if there is no node?
      // if node is in per_key but not here - it's a consistency error
      let node = self.nodes.get(head_id).expect("if not here - huge consistency problem");
      (node.reg_id, node.fiber_id)
    };

    // TODO: should I panic here if there is no regs?
    // if node is in nodes and per_key but not here - it's a consistency error
    let reg = self.regs.try_remove(reg_id).expect("if not here - huge consistency problem");

    let winner_resume: ArmResume = {
      let mut to_return: Option<ArmResume> = None;
      for arm in reg.arms {
        self.list_unlink(&arm.key, arm.node_id);
        if arm.node_id == head_id {
          to_return = Some(arm.resume);
        }
      }
      to_return.expect("if not here - huge consistency problem ಠ_ಠ")
    };

    Some(WakeOutcome { fiber_id, bind: winner_resume.bind, next: winner_resume.next })
  }

  /// appends a waiter node to the end of the per-source FIFO list
  /// O(1)
  /// requires node.prev/next are None
  fn list_push_back(
    &mut self,
    key: &WaitKey,
    node_id: WaitNodeId,
  ) {
    let wl = self.per_key.entry(key.clone()).or_default();
    match wl.tail.take() {
      // we end up here if list for this particular WaitKey is empty
      // then this single node becomes head and tail and we don't need to modify it in nodes
      None => {
        wl.head = Some(node_id);
        wl.tail = Some(node_id);
      }
      // here if FIFO for a particular WaitKey is not empty
      // does:
      // - links old tail -> node
      // - links node -> old tal
      // - updates list tail
      Some(tail_id) => {
        if let Some(tail_node) = self.nodes.get_mut(tail_id) {
          tail_node.next = Some(node_id);
        }
        if let Some(cur_node) = self.nodes.get_mut(node_id) {
          cur_node.prev = Some(tail_id);
        }
        wl.tail = Some(node_id);
      }
    }
  }

  /// - Unlinks a node from intrusive doubly linked `self.nodes` list and removes its slab entry
  /// - Unlinks a node from a `self.per_key` FIFO list where it's registered for a given WaitKey
  /// - If WaitList becomes empty - removes it from `self.per_key`
  /// - amortized O(1)
  fn list_unlink(
    &mut self,
    key: &WaitKey,
    node_id: WaitNodeId,
  ) {
    // Capture prev/next
    let (prev, next) = match self.nodes.try_remove(node_id) {
      Some(n) => (n.prev, n.next),
      None => return,
    };

    // Update neighbors
    if let Some(p) = prev {
      if let Some(pn) = self.nodes.get_mut(p) {
        pn.next = next;
      }
    }
    if let Some(n) = next {
      if let Some(nn) = self.nodes.get_mut(n) {
        nn.prev = prev;
      }
    }

    // Update list head/tail
    let mut remove_list_entry = false;
    if let Some(wl) = self.per_key.get_mut(key) {
      if wl.head == Some(node_id) {
        wl.head = next;
      }
      if wl.tail == Some(node_id) {
        wl.tail = prev;
      }
      remove_list_entry = wl.head.is_none() && wl.tail.is_none();
    }

    // Remove empty list entry to keep map compact
    if remove_list_entry {
      self.per_key.remove(key);
    }
  }

  /// Cancels a specific in-flight select by its registration id; returns number of arms unlinked.
  /// O(selected_arms) ~ O(1)
  pub fn cancel_by_registered_select_id(
    &mut self,
    id: RegisteredSelectId,
  ) -> usize {
    let mut removed = 0;
    let r_try = self.regs.try_remove(id.0);
    if let Some(reg) = r_try {
      for arm in reg.arms {
        self.list_unlink(&arm.key, arm.node_id);
        removed += 1;
      }
    }
    removed
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn fifo_two_fibers_on_same_queue() {
    let mut wr = WaitRegistry::default();
    wr.register_select(
      1,
      vec![SelectArm::Queue { queue_name: "q".to_string(), bind: "a".to_string(), next: State::GlobalAddEntry }],
    );
    wr.register_select(
      2,
      vec![SelectArm::Queue { queue_name: "q".to_string(), bind: "b".to_string(), next: State::GlobalDivEntry }],
    );

    let out1 = wr.wake_one(&WaitKey::Queue("q".to_string())).expect("wake #1");
    assert_eq!(out1.fiber_id, 1);
    assert_eq!(out1.bind.as_deref(), Some("a"));
    assert_eq!(out1.next, State::GlobalAddEntry);

    let out2 = wr.wake_one(&WaitKey::Queue("q".to_string())).expect("wake #2");
    assert_eq!(out2.fiber_id, 2);
    assert_eq!(out2.bind.as_deref(), Some("b"));
    assert_eq!(out2.next, State::GlobalDivEntry);

    assert!(wr.per_key.get(&WaitKey::Queue("q".to_string())).is_none());
  }

  #[test]
  fn mixed_arms_across_keys() {
    let mut wr = WaitRegistry::default();
    wr.register_select(
      1,
      vec![
        SelectArm::Queue { queue_name: "a".to_string(), bind: "x".to_string(), next: State::Idle },
        SelectArm::Queue { queue_name: "b".to_string(), bind: "y".to_string(), next: State::Completed },
      ],
    );
    wr.register_select(
      2,
      vec![SelectArm::Queue { queue_name: "a".to_string(), bind: "z".to_string(), next: State::Completed }],
    );

    // Resolve on b -> wakes fiber 1 via b arm, and removes its a arm
    let out = wr.wake_one(&WaitKey::Queue("b".to_string())).expect("wake on b");
    assert_eq!(out.fiber_id, 1);
    assert_eq!(out.bind.as_deref(), Some("y"));
    assert_eq!(out.next, State::Completed);

    // Now resolving on a -> wakes fiber 2, not 3
    let out2 = wr.wake_one(&WaitKey::Queue("a".to_string())).expect("wake on a");
    assert_eq!(out2.fiber_id, 2);
    assert_eq!(out2.bind.as_deref(), Some("z"));
    assert_eq!(out2.next, State::Completed);
  }

  #[test]
  fn cancel_by_select_id_removes_only_that_fiber() {
    let mut wr = WaitRegistry::default();
    // Fiber 1 on q1 and q2
    let id1 = wr.register_select(
      1,
      vec![
        SelectArm::Queue { queue_name: "q1".to_string(), bind: "a".to_string(), next: State::Idle },
        SelectArm::Queue { queue_name: "q2".to_string(), bind: "b".to_string(), next: State::Idle },
      ],
    );
    // Fiber 2 on q2 only
    wr.register_select(
      2,
      vec![SelectArm::Queue { queue_name: "q2".to_string(), bind: "c".to_string(), next: State::Completed }],
    );

    let removed = wr.cancel_by_registered_select_id(id1);
    assert_eq!(removed, 2);

    // q1 should have no waiters
    assert!(wr.wake_one(&WaitKey::Queue("q1".to_string())).is_none());

    // q2 should wake fiber 2
    let out = wr.wake_one(&WaitKey::Queue("q2".to_string())).expect("wake fiber 2 on q2");
    assert_eq!(out.fiber_id, 2);
    assert_eq!(out.bind.as_deref(), Some("c"));
    assert_eq!(out.next, State::Completed);
  }

  #[test]
  fn remove_last_node_cleans_everything() {
    let mut registry = WaitRegistry::default();
    registry.register_select(
      100500,
      vec![
        SelectArm::Queue { queue_name: "q1".to_string(), bind: "var1".to_string(), next: State::GlobalDivEntry },
        SelectArm::Queue { queue_name: "q2".to_string(), bind: "var2".to_string(), next: State::GlobalAddEntry },
        SelectArm::Future { future_id: FutureLabel::new("f1"), bind: Some("var3".to_string()), next: State::Completed },
      ],
    );

    let result = registry.wake_one(&WaitKey::Queue("q2".to_string()));
    assert_eq!(
      Some(WakeOutcome { fiber_id: 100500, bind: Some("var2".to_string()), next: State::GlobalAddEntry }),
      result
    );
    assert!(registry.nodes.len() == 0, "{:?}", registry.nodes);
    assert!(registry.per_key.len() == 0, "{:?}", registry.nodes);
    assert!(registry.regs.len() == 0, "{:?}", registry.nodes);
  }

  #[test]
  fn empty_wake_returns_none() {
    let mut wr = WaitRegistry::default();
    assert!(wr.wake_one(&WaitKey::Queue("q".to_string())).is_none());
    assert!(wr.per_key.is_empty());
    assert_eq!(wr.nodes.len(), 0);
    assert_eq!(wr.regs.len(), 0);
  }

  #[test]
  fn fifo_three_fibers_on_same_queue() {
    let mut wr = WaitRegistry::default();
    let q = "q".to_string();
    wr.register_select(1, vec![SelectArm::Queue { queue_name: q.clone(), bind: "a".to_string(), next: State::Idle }]);
    wr.register_select(2, vec![SelectArm::Queue { queue_name: q.clone(), bind: "b".to_string(), next: State::Idle }]);
    wr.register_select(3, vec![SelectArm::Queue { queue_name: q.clone(), bind: "c".to_string(), next: State::Idle }]);

    let out1 = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    let out2 = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    let out3 = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    assert_eq!((out1.fiber_id, out1.bind), (1, Some("a".to_string())));
    assert_eq!((out2.fiber_id, out2.bind), (2, Some("b".to_string())));
    assert_eq!((out3.fiber_id, out3.bind), (3, Some("c".to_string())));
    assert!(wr.wake_one(&WaitKey::Queue(q)).is_none());
  }

  #[test]
  fn cancel_middle_waiter_by_id() {
    let mut wr = WaitRegistry::default();
    let q = "q".to_string();
    wr.register_select(1, vec![SelectArm::Queue { queue_name: q.clone(), bind: "a".to_string(), next: State::Idle }]);
    let id2 =
      wr.register_select(2, vec![SelectArm::Queue { queue_name: q.clone(), bind: "b".to_string(), next: State::Idle }]);
    wr.register_select(3, vec![SelectArm::Queue { queue_name: q.clone(), bind: "c".to_string(), next: State::Idle }]);

    // Remove the middle waiter (fiber 2)
    let removed = wr.cancel_by_registered_select_id(id2);
    assert_eq!(removed, 1);

    let out1 = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    let out2 = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    assert_eq!(out1.fiber_id, 1);
    assert_eq!(out2.fiber_id, 3);
    assert!(wr.wake_one(&WaitKey::Queue(q)).is_none());
  }

  #[test]
  fn duplicate_arms_same_queue() {
    let mut wr = WaitRegistry::default();
    // Fiber 1 registers two arms on the same queue
    wr.register_select(
      1,
      vec![
        SelectArm::Queue { queue_name: "q".to_string(), bind: "x".to_string(), next: State::Idle },
        SelectArm::Queue { queue_name: "q".to_string(), bind: "y".to_string(), next: State::Idle },
      ],
    );
    // Fiber 2 registers one arm on the same queue
    wr.register_select(
      2,
      vec![SelectArm::Queue { queue_name: "q".to_string(), bind: "z".to_string(), next: State::Idle }],
    );

    // First wake should wake fiber 1 and remove both of its arms
    let out = wr.wake_one(&WaitKey::Queue("q".to_string())).unwrap();
    assert_eq!(out.fiber_id, 1);
    // Next wake should pick fiber 2 (not the second arm of fiber 1)
    let out2 = wr.wake_one(&WaitKey::Queue("q".to_string())).unwrap();
    assert_eq!(out2.fiber_id, 2);
    assert!(wr.wake_one(&WaitKey::Queue("q".to_string())).is_none());
  }

  #[test]
  fn mixed_keys_fairness_ordering() {
    let mut wr = WaitRegistry::default();
    // Queue a: fibers 1,2
    wr.register_select(
      1,
      vec![SelectArm::Queue { queue_name: "a".to_string(), bind: "a1".to_string(), next: State::Idle }],
    );
    wr.register_select(
      2,
      vec![SelectArm::Queue { queue_name: "a".to_string(), bind: "a2".to_string(), next: State::Idle }],
    );
    // Queue b: fibers 3,4
    wr.register_select(
      3,
      vec![SelectArm::Queue { queue_name: "b".to_string(), bind: "b1".to_string(), next: State::Idle }],
    );
    wr.register_select(
      4,
      vec![SelectArm::Queue { queue_name: "b".to_string(), bind: "b2".to_string(), next: State::Idle }],
    );

    let a1 = wr.wake_one(&WaitKey::Queue("a".to_string())).unwrap();
    let b1 = wr.wake_one(&WaitKey::Queue("b".to_string())).unwrap();
    let a2 = wr.wake_one(&WaitKey::Queue("a".to_string())).unwrap();
    let b2 = wr.wake_one(&WaitKey::Queue("b".to_string())).unwrap();

    assert_eq!((a1.fiber_id, a2.fiber_id, b1.fiber_id, b2.fiber_id), (1, 2, 3, 4));
  }

  #[test]
  fn bind_variants_some_and_none() {
    let mut wr = WaitRegistry::default();
    // Same fiber waiting on two different keys with and without bind
    wr.register_select(
      42,
      vec![
        SelectArm::Queue { queue_name: "a".to_string(), bind: "bind_a".to_string(), next: State::Idle },
        SelectArm::Future { future_id: FutureLabel::new("fy"), bind: None, next: State::Completed },
      ],
    );

    // Wake future arm first
    let fy = WaitKey::Future(FutureId::from_label(FutureLabel::new("fy"), 42));
    let out = wr.wake_one(&fy).unwrap();
    assert_eq!(out.fiber_id, 42);
    assert_eq!(out.bind, None);
    assert_eq!(out.next, State::Completed);
  }

  #[test]
  fn cancel_noop_when_already_removed() {
    let mut wr = WaitRegistry::default();
    let id = wr.register_select(
      72,
      vec![SelectArm::Queue { queue_name: "q".to_string(), bind: "b".to_string(), next: State::Idle }],
    );
    let removed1 = wr.cancel_by_registered_select_id(id);
    assert_eq!(removed1, 1);
    let removed2 = wr.cancel_by_registered_select_id(id);
    assert_eq!(removed2, 0);
  }

  #[test]
  fn reuse_after_wake_registers_new_orders_correctly() {
    let mut wr = WaitRegistry::default();
    let q = "q".to_string();
    wr.register_select(1, vec![SelectArm::Queue { queue_name: q.clone(), bind: "a".to_string(), next: State::Idle }]);
    wr.register_select(2, vec![SelectArm::Queue { queue_name: q.clone(), bind: "b".to_string(), next: State::Idle }]);
    // Wake both
    let _ = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    let _ = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    // Register new ones after previous cleared
    wr.register_select(3, vec![SelectArm::Queue { queue_name: q.clone(), bind: "c".to_string(), next: State::Idle }]);
    wr.register_select(4, vec![SelectArm::Queue { queue_name: q.clone(), bind: "d".to_string(), next: State::Idle }]);
    let out1 = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    let out2 = wr.wake_one(&WaitKey::Queue(q.clone())).unwrap();
    assert_eq!((out1.fiber_id, out2.fiber_id), (3, 4));
  }
}
