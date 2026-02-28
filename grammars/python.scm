; Tree-sitter query for Python import resolution.
; M1 implementation.

; import foo / import foo.bar
(import_statement
  name: (dotted_name) @import.specifier) @import.decl

; from foo import bar / from foo.bar import baz, qux
(import_from_statement
  module_name: (dotted_name) @import.specifier) @import.decl.from

; Named symbols from from-import: from foo import bar, baz
(import_from_statement
  name: (dotted_name) @import.symbol)

; Aliased symbol: from foo import bar as b
(import_from_statement
  name: (aliased_import
    name: (dotted_name) @import.symbol))

; from . import bar  (relative, no module name)
(import_from_statement
  module_name: (relative_import) @import.relative) @import.decl.relative

; Function definitions (including async)
(function_definition
  name: (identifier) @function.name) @function.decl

; Class definitions
(class_definition
  name: (identifier) @class.name) @class.decl
