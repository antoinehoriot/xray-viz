package com.example;

import java.util.List;
import java.util.ArrayList;

/**
 * Greetable interface for objects that can produce greetings.
 */
interface Greetable {
    String greet(String name);
}

/**
 * Simple class demonstrating Java parser extraction.
 */
public class Simple implements Greetable {

    private final String prefix;

    public Simple(String prefix) {
        this.prefix = prefix;
    }

    @Override
    public String greet(String name) {
        return prefix + ", " + name + "!";
    }

    public List<String> greetAll(List<String> names) {
        List<String> results = new ArrayList<>();
        for (String name : names) {
            results.add(greet(name));
        }
        return results;
    }

    public static void main(String[] args) {
        Simple s = new Simple("Hello");
        System.out.println(s.greet("World"));
    }
}
