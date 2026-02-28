package main

import (
	"fmt"
	"strings"
)

// Greeter is a type that knows how to greet.
type Greeter struct {
	Name string
}

// String returns the string representation of a Greeter.
func (g Greeter) String() string {
	return "Greeter(" + g.Name + ")"
}

// greet returns a greeting for the given name.
func greet(name string) string {
	return "Hello, " + strings.TrimSpace(name) + "!"
}

func main() {
	g := Greeter{Name: "World"}
	fmt.Println(greet(g.Name))
	fmt.Println(g.String())
}
