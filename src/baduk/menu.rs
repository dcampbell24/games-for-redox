//! This module provides utilities for creating command line menus.

use std::fmt;
use std::io;
use std::str::FromStr;

pub struct Menu<T> {
    pub prompt: String,
    pub default: usize,
    pub options: Vec<T>,
}

impl<T: Clone + fmt::Display> Menu<T> {
    pub fn select_option(&self) -> T {
        assert!(self.options.len() > 0);

        println!("{}", &self.prompt);
        for (i, option) in self.options.iter().enumerate() {
            if i == self.default {
                println!("{}. [{}]", i, option);
            } else {
                println!("{}. {}", i, option);
            }
        }

        let stdin = io::stdin();
        let mut buf = String::new();
        let error_string = format!(
            "error: expected integer in range [0, {}]", self.options.len() - 1
        );

        loop {
            let _bytes_read = stdin.read_line(&mut buf).unwrap();
            {
                let line = buf.trim();
                if line == "" {
                    return self.options[self.default].clone();
                }

                match usize::from_str(line) {
                    Ok(num) => match self.options.get(num) {
                        Some(item) => {
                            println!("");
                            return item.clone()
                        },
                        None => (),
                    },
                    Err(_) => (),
                }
            }
            println!("{}", &error_string);
            buf.clear();
        }
    }
}
