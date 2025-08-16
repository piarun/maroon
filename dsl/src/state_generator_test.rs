use crate::{parser, state_generator::states_from_program};

#[test]
fn test_function_calls_states() {
  let input = r#"
        fn factorial(n: i32) -> i32 {
            if n == 0 {
                return 1
            } else if n == 1 {
                return 1
            }

            return n * factorial(n - 1)
        }

        fn delayed(t: i64, message: String){
          sleep(t)
          print(message)
        }
    "#;

  let program = parser::parse_program(input);
  assert!(program.is_ok());

  let program = program.unwrap();
  let expected_states = vec![
    // functions
    "FactorialEntry".to_string(),
    "FactorialRecursiveCall".to_string(),
    "FactorialDone".to_string(),
    "DelayedEntry".to_string(),
    "DelayedCallSleep".to_string(),
    "DelayedCallPrint".to_string(),
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
