; Tree-sitter query for TypeScript/JavaScript import and export resolution.
; M1 implementation: full import/export extraction.

; Static import declarations: import { foo } from "bar"
(import_statement
  source: (string) @import.specifier) @import.decl

; Dynamic imports: import("bar")
(call_expression
  function: (import)
  arguments: (arguments (string) @import.dynamic.specifier)) @import.dynamic

; Export declarations
(export_statement) @export.decl

; Function declarations
(function_declaration
  name: (identifier) @function.name) @function.decl

; Arrow function assigned to variable
(variable_declarator
  name: (identifier) @function.name
  value: [(arrow_function) (function)]) @function.decl

; Class declarations
(class_declaration
  name: (identifier) @class.name) @class.decl
