# Data Dictionary

### `MaroonIRVarRegular`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `name` | String |
| `type` | String |
| `init` | String |


### `MaroonIRVarFunctionArg`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `name` | String |
| `type` | String |


### `MaroonIRVarEnumCaseCapture`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `name` | String |
| `key` | String |
| `src` | String |


### `MaroonIRVar`
Algebraic type, `MaroonIRVarRegular` or `MaroonIRVarFunctionArg` or `MaroonIRVarEnumCaseCapture`


### `MaroonIRStmt`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `stmt` | String |


### `MaroonIRIf`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `cond` | String |
| `yes` | Algebraic `MaroonIRStmt` / `MaroonIRIf` / `MaroonIRBlock` / `MaroonIRMatchEnumStmt` / `MaroonIRBlockPlaceholder` (a.k.a. `MaroonIRStmtOrBlock`) |
| `no` | Algebraic `MaroonIRStmt` / `MaroonIRIf` / `MaroonIRBlock` / `MaroonIRMatchEnumStmt` / `MaroonIRBlockPlaceholder` (a.k.a. `MaroonIRStmtOrBlock`) |


### `MaroonIRMatchEnumStmtArm`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `key` | `null` or String |
| `capture` | `null` or String |
| `code` | `MaroonIRBlock` |


### `MaroonIRMatchEnumStmt`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `var` | String |
| `arms` | Array of `MaroonIRMatchEnumStmtArm` |


### `MaroonIRBlockPlaceholder`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `_idx` | Integer (32-bit unsigned) |


### `MaroonIRStmtOrBlock`
Algebraic type, `MaroonIRStmt` or `MaroonIRIf` or `MaroonIRBlock` or `MaroonIRMatchEnumStmt` or `MaroonIRBlockPlaceholder`


### `MaroonIRBlock`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `vars` | Array of Algebraic `MaroonIRVarRegular` / `MaroonIRVarFunctionArg` / `MaroonIRVarEnumCaseCapture` (a.k.a. `MaroonIRVar`) |
| `code` | Array of Algebraic `MaroonIRStmt` / `MaroonIRIf` / `MaroonIRBlock` / `MaroonIRMatchEnumStmt` / `MaroonIRBlockPlaceholder` (a.k.a. `MaroonIRStmtOrBlock`) |


### `MaroonIRFunction`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `ret` | `null` or String |
| `args` | Array of String |
| `body` | `MaroonIRBlock` |


### `MaroonIRFiber`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `functions` | Ordered map of String into `MaroonIRFunction` |


### `MaroonIRTypeDefStructField`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `name` | String |
| `type` | String |


### `MaroonIRTypeDefStruct`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `fields` | Array of `MaroonIRTypeDefStructField` |


### `MaroonIRTypeDefEnumCase`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `key` | String |
| `type` | String |


### `MaroonIRTypeDefEnum`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `cases` | Array of `MaroonIRTypeDefEnumCase` |


### `MaroonIRTypeDefOptional`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `type` | String |


### `MaroonIRTypeDef`
Algebraic type, `MaroonIRTypeDefStruct` or `MaroonIRTypeDefEnum` or `MaroonIRTypeDefOptional`


### `MaroonIRType`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `def` | Algebraic `MaroonIRTypeDefStruct` / `MaroonIRTypeDefEnum` / `MaroonIRTypeDefOptional` (a.k.a. `MaroonIRTypeDef`) |


### `MaroonIRNamespace`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `fibers` | Ordered map of String into `MaroonIRFiber` |
| `types` | Ordered map of String into `MaroonIRType` |


### `MaroonTestCaseRunFiber`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `maroon` | String |
| `fiber` | String |
| `golden_output` | Array of String |


### `MaroonTestCaseFiberShouldThrow`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `line` | Integer (32-bit unsigned) |
| `maroon` | String |
| `fiber` | String |
| `error` | String |


### `MaroonTestCase`
Algebraic type, `MaroonTestCaseRunFiber` or `MaroonTestCaseFiberShouldThrow`


### `MaroonIRScenarios`
| **Field** | **Type** | **Description** |
| ---: | :--- | :--- |
| `src` | String | The source `.mrn` file. |
| `maroon` | Ordered map of String into `MaroonIRNamespace` |
| `tests` | Array of Algebraic `MaroonTestCaseRunFiber` / `MaroonTestCaseFiberShouldThrow` (a.k.a. `MaroonTestCase`) |

