; Tree-sitter query for Python import resolution.
; M1 implementation.

; import foo
(import_statement
  name: (dotted_name) @import.specifier) @import.decl

; from foo import bar
(import_from_statement
  module_name: (dotted_name) @import.specifier
  name: [(dotted_name) (aliased_import)] @import.symbol) @import.decl

; from . import bar (relative import)
(import_from_statement
  module_name: (relative_import) @import.relative) @import.decl.relative

; Function definitions
(function_definition
  name: (identifier) @function.name) @function.decl

; Class definitions
(class_definition
  name: (identifier) @class.name) @class.decl
