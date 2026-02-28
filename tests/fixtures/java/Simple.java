package com.example;

import java.util.ArrayList;
import java.util.List;
import java.io.IOException;

/**
 * A simple Java class for testing the tree-sitter parser.
 */
public class Simple {

    private String name;

    public Simple(String name) {
        this.name = name;
    }

    public String greet() {
        return "Hello, " + this.name + "!";
    }

    public List<String> getItems() throws IOException {
        List<String> items = new ArrayList<>();
        items.add("item1");
        items.add("item2");
        return items;
    }

    public static void main(String[] args) {
        Simple s = new Simple("World");
        System.out.println(s.greet());
    }
}
