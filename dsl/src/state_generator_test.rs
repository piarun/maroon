use crate::{parser, state_generator::states_from_program};

#[test]
fn test_recursive_function_states() {
  let input = r#"
        fn recFunc(number: i32) -> i32 { 
          if sync is_odd(number) {
            return recFunc(number - 1)
          } else {
            return recFunc(number - 2)
          }
        }
    "#;

  let program = parser::parse_program(input);
  assert!(program.is_ok(), "{}", program.unwrap_err());

  let program = program.unwrap();
  let expected_states = vec![
    "RecFuncEntry".to_string(),
    "RecFuncRecursiveCall0".to_string(),
    "RecFuncRecursiveCall1".to_string(),
    "RecFuncDone".to_string(),
  ];

  assert_eq!(expected_states, states_from_program(&program))
}

#[test]
fn test_call_function_states() {
  let input = r#"
        fn delayed(t: i64, message: String){
          sleep(t)
          print(message)
          sleep(t * 2)
          print(message)
        }
    "#;

  let program = parser::parse_program(input);
  assert!(program.is_ok());

  let program = program.unwrap();
  let expected_states = vec![
    "DelayedEntry".to_string(),
    "DelayedCallSleep0".to_string(),
    "DelayedCallPrint0".to_string(),
    "DelayedCallSleep1".to_string(),
    "DelayedCallPrint1".to_string(),
    "DelayedDone".to_string(),
  ];

  assert_eq!(expected_states, states_from_program(&program))
}

#[test]
fn test_expression_in_statement_cross_fiber() {
  let input = r#"
        struct User {
          id: String,
          email: String,
          age: i32,
        }

        var users: map[String]User = {}

        fn findUserById(id: String) -> User {
          return users.get(id)
        }

        fn createNewUser(email: String, age: i32) {
          let id: String = sync generateId()
          users.set(id, User { id: id, email: email, age: age })
        }
    "#;

  let program = parser::parse_program(input);
  assert!(program.is_ok(), "{}", program.unwrap_err());

  let program = program.unwrap();
  let expected_states = vec![
    // all global variables are named as VarName+Storage => users -> UsersStorage
    "UsersStorageIdle".to_string(), // will get here after all get/create requests are processed
    "UsersStorageGetItemRequest".to_string(), // in global variable we have get/create by default
    "UsersStorageCreateItemRequest".to_string(),
    //
    "FindUserByIdEntry".to_string(),
    "FindUserByIdGetUsersRequest".to_string(), // func name + fiber_func_name + global var(fiber) name + Request
    "FindUserByIdGetUsersGot".to_string(),     // func name + fiber_func_name + global var(fiber) name + Got
    "FindUserByIdDone".to_string(),
    //
    "CreateNewUserEntry".to_string(),
    "CreateNewUserCreateUsersRequest".to_string(), // there is no CreateNewUserCallGenerateId because it's marked as sync call
    "CreateNewUserCreateUsersGot".to_string(),
    "CreateNewUserDone".to_string(),
  ];

  assert_eq!(expected_states, states_from_program(&program))
}
