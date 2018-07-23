/*
 * Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
 *
 * This file is part of zinc64.
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::cell::RefCell;
use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

use core::Cpu;

enum Operator {
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Operator::Equal => write!(f, "=="),
            &Operator::NotEqual => write!(f, "!="),
            &Operator::Greater => write!(f, ">"),
            &Operator::GreaterEqual => write!(f, ">="),
            &Operator::Less => write!(f, "<"),
            &Operator::LessEqual => write!(f, "<="),
        }
    }
}

enum Reg {
    A,
    X,
    Y,
    P,
    SP,
    PC,
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Reg::A => write!(f, "A"),
            &Reg::X => write!(f, "X"),
            &Reg::Y => write!(f, "Y"),
            &Reg::P => write!(f, "P"),
            &Reg::SP => write!(f, "SP"),
            &Reg::PC => write!(f, "PC"),
        }
    }
}

enum Value {
    Constant(u16),
    Register(Reg),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Value::Constant(val) if val <= 0xff => write!(f, "{:02x}", val),
            &Value::Constant(val) => write!(f, "{:04x}", val),
            &Value::Register(ref reg) => write!(f, "{}", reg),
        }
    }
}

pub struct Condition {
    op: Operator,
    reg: Reg,
    val: Value,
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.reg, self.op, self.val)
    }
}

impl Condition {
    pub fn parse(expr: &str, radix: Option<u32>) -> Result<Condition, String> {
        let parser = Parser::new(radix.unwrap_or(16));
        parser.parse(expr)
    }

    pub fn eval(&self, cpu: &RefCell<Cpu>) -> bool {
        match self.op {
            Operator::Equal => self.eval_reg(&self.reg, cpu) == self.eval_val(&self.val, cpu),
            Operator::NotEqual => self.eval_reg(&self.reg, cpu) != self.eval_val(&self.val, cpu),
            Operator::Greater => self.eval_reg(&self.reg, cpu) > self.eval_val(&self.val, cpu),
            Operator::GreaterEqual => {
                self.eval_reg(&self.reg, cpu) >= self.eval_val(&self.val, cpu)
            }
            Operator::Less => self.eval_reg(&self.reg, cpu) < self.eval_val(&self.val, cpu),
            Operator::LessEqual => self.eval_reg(&self.reg, cpu) <= self.eval_val(&self.val, cpu),
        }
    }

    fn eval_reg(&self, reg: &Reg, cpu: &RefCell<Cpu>) -> u16 {
        match reg {
            &Reg::A => cpu.borrow().get_a() as u16,
            &Reg::X => cpu.borrow().get_x() as u16,
            &Reg::Y => cpu.borrow().get_y() as u16,
            &Reg::P => cpu.borrow().get_p() as u16,
            &Reg::SP => cpu.borrow().get_sp() as u16,
            &Reg::PC => cpu.borrow().get_pc(),
        }
    }

    fn eval_val(&self, val: &Value, cpu: &RefCell<Cpu>) -> u16 {
        match val {
            &Value::Constant(value) => value,
            &Value::Register(ref reg) => self.eval_reg(reg, cpu),
        }
    }
}

struct Parser {
    radix: u32,
}

impl Parser {
    pub fn new(radix: u32) -> Self {
        Parser { radix }
    }

    pub fn parse(&self, expr: &str) -> Result<Condition, String> {
        let mut tokenizer = Tokenizer::new(expr.chars());
        let reg = match tokenizer.next() {
            Some(Token::Atom(token)) => self.parse_reg(token.as_str()),
            _ => Err(format!("Invalid expression {}", expr)),
        }?;
        let op = match tokenizer.next() {
            Some(Token::Op(token)) => self.parse_op(token.as_str()),
            _ => Err(format!("Invalid expression {}", expr)),
        }?;
        let val = match tokenizer.next() {
            Some(Token::Atom(token)) => self.parse_val(token.as_str()),
            _ => Err(format!("Invalid expression {}", expr)),
        }?;
        let condition = Condition { op, reg, val };
        Ok(condition)
    }

    fn parse_num(&self, num: &str) -> Result<u16, String> {
        u16::from_str_radix(num, self.radix).map_err(|_| format!("Invalid number {}", num))
    }

    fn parse_op(&self, op: &str) -> Result<Operator, String> {
        match op {
            "==" => Ok(Operator::Equal),
            "!=" => Ok(Operator::NotEqual),
            ">" => Ok(Operator::Greater),
            ">=" => Ok(Operator::GreaterEqual),
            "<" => Ok(Operator::Less),
            "<=" => Ok(Operator::LessEqual),
            _ => Err(format!("Invalid op {}", op)),
        }
    }

    fn parse_reg(&self, reg: &str) -> Result<Reg, String> {
        match reg {
            "a" | "A" => Ok(Reg::A),
            "x" | "X" => Ok(Reg::X),
            "y" | "Y" => Ok(Reg::Y),
            "p" | "P" => Ok(Reg::P),
            "sp" | "SP" => Ok(Reg::SP),
            "pc" | "PC" => Ok(Reg::PC),
            _ => Err(format!("Invalid register {}", reg)),
        }
    }

    fn parse_val(&self, val: &str) -> Result<Value, String> {
        match self.parse_reg(val) {
            Ok(reg) => Ok(Value::Register(reg)),
            Err(_) => self.parse_num(val).map(|v| Value::Constant(v)),
        }
    }
}

#[derive(Eq, PartialEq)]
pub enum Token {
    Atom(String),
    Op(String),
}

pub struct Tokenizer<'a> {
    iter: Peekable<Chars<'a>>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: Chars<'a>) -> Tokenizer<'a> {
        Tokenizer {
            iter: input.peekable(),
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        match *self.iter.peek().unwrap_or(&'⊥') {
            c if c.is_alphanumeric() => Some(Token::Atom(consume_while(&mut self.iter, |c| {
                c.is_alphanumeric()
            }))),
            c if is_symbol(c) => Some(Token::Op(consume_while(&mut self.iter, |c| is_symbol(c)))),
            c if c.is_whitespace() => self.next(),
            '⊥' => None,
            _ => self.next(),
        }
    }
}

fn consume_while<F>(iter: &mut Peekable<Chars>, predicate: F) -> String
where
    F: Fn(char) -> bool,
{
    let mut s = String::new();
    while let Some(&c) = iter.peek() {
        if !predicate(c) {
            break;
        }
        iter.next();
        s.push(c);
    }
    s
}

fn is_symbol(c: char) -> bool {
    let result = match c {
        '<' | '=' | '>' | '!' => true,
        _ => false,
    };
    result
}
