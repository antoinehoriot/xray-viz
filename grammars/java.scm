; Tree-sitter query for Java import resolution.
; M3 implementation.

; import statements: import com.example.Foo;
(import_declaration
  (scoped_identifier) @import.specifier) @import.decl

; Class declarations
(class_declaration
  name: (identifier) @class.name) @class.decl

; Interface declarations
(interface_declaration
  name: (identifier) @interface.name) @interface.decl

; Method declarations
(method_declaration
  name: (identifier) @method.name) @method.decl

; Package declarations
(package_declaration
  (scoped_identifier) @package.name) @package.decl
