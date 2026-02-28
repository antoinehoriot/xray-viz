; Tree-sitter query for Go import resolution.
; M3 implementation.

; Single import: import "fmt"
(import_spec
  path: (interpreted_string_literal) @import.specifier) @import.decl

; Function declarations
(function_declaration
  name: (identifier) @function.name) @function.decl

; Method declarations
(method_declaration
  name: (field_identifier) @method.name) @method.decl

; Type declarations
(type_spec
  name: (type_identifier) @type.name) @type.decl
