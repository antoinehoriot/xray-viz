; Tree-sitter query for Rust module and use resolution.
; M1 implementation.

; use declarations: use foo::bar
(use_declaration
  argument: (_) @import.specifier) @import.decl

; mod declarations: mod foo;
(mod_item
  name: (identifier) @mod.name) @mod.decl

; extern crate: extern crate foo;
(extern_crate_declaration
  name: (identifier) @extern.crate) @extern.decl

; Function items
(function_item
  name: (identifier) @function.name) @function.decl

; Struct items
(struct_item
  name: (type_identifier) @struct.name) @struct.decl

; Enum items
(enum_item
  name: (type_identifier) @enum.name) @enum.decl

; Trait items
(trait_item
  name: (type_identifier) @trait.name) @trait.decl
