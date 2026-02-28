package main

import (
	"fmt"
	"os"
)

// Person is a simple struct type.
type Person struct {
	Name string
	Age  int
}

// Greet returns a greeting string for a person.
func Greet(p Person) string {
	return fmt.Sprintf("Hello, %s!", p.Name)
}

// (Receiver method on Person)
func (p Person) String() string {
	return fmt.Sprintf("%s (%d)", p.Name, p.Age)
}

func main() {
	args := os.Args
	_ = args

	p := Person{Name: "World", Age: 42}
	fmt.Println(Greet(p))
}
