; Tree-sitter query for TypeScript/JavaScript import and export resolution.
; M1 implementation: full import/export extraction.
; Compatible with tree-sitter-typescript 0.21.x
;
; NOTE: In TypeScript, class names are (type_identifier), NOT (identifier).

; Static import declarations: import ... from "bar"
(import_statement
  source: (string) @import.specifier) @import.decl

; Named import symbols: import { foo, bar } from "baz"
(import_statement
  (import_clause
    (named_imports
      (import_specifier
        name: (identifier) @import.symbol))))

; Dynamic imports: import("bar")
(call_expression
  function: (import)
  arguments: (arguments (string) @import.dynamic.specifier)) @import.dynamic

; Re-export with source: export * from "bar" / export { foo } from "bar"
(export_statement
  source: (string) @export.source) @export.decl.reexport

; Export of named function: export function foo() {}
(export_statement
  declaration: (function_declaration
    name: (identifier) @export.name)) @export.decl.named

; Export of named class: export class Foo {}
; NOTE: class name is type_identifier in TypeScript grammar
(export_statement
  declaration: (class_declaration
    name: (type_identifier) @export.name)) @export.decl.named

; Export of lexical declaration: export const foo = ...
(export_statement
  declaration: (lexical_declaration
    (variable_declarator
      name: (identifier) @export.name))) @export.decl.named

; Function declarations
(function_declaration
  name: (identifier) @function.name) @function.decl

; Arrow function assigned to variable
(variable_declarator
  name: (identifier) @function.name
  value: (arrow_function)) @function.decl

; Function expression assigned to variable
(variable_declarator
  name: (identifier) @function.name
  value: (function_expression)) @function.decl

; Class declarations — type_identifier for class names in TypeScript
(class_declaration
  name: (type_identifier) @class.name) @class.decl
