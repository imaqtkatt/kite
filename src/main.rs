use std::{
  collections::HashMap, iter::Peekable, marker::PhantomData, str::Chars,
};

#[derive(Default, Debug)]
pub enum JsonValue {
  String(String),
  Number(f64),
  Bool(bool),
  Object(HashMap<String, Id<JsonValue>>),
  List(Vec<Id<JsonValue>>),
  #[default]
  Null,
}

struct Lex<'json> {
  code: Peekable<Chars<'json>>,
}

impl<'json> Lex<'json> {
  fn new(code: &'json str) -> Self {
    let code = code.chars().peekable();
    Self { code }
  }
}

#[derive(Debug)]
enum Token {
  Str(String),
  Num(f64),
  False,
  True,
  Null,
  LBrace,
  RBrace,
  LBracket,
  RBracket,
  Comma,
  Colon,
  Eof,
  IllegalIdent,
}

impl<'json> Lex<'json> {
  fn next_token(&mut self) -> Token {
    if let Some(chr) = self.code.peek() {
      match chr {
        ' ' | '\n' | '\t' | '\r' => {
          self.code.next();
          self.next_token()
        }
        '"' => self.str(),
        ':' => self.just(Token::Colon),
        ',' => self.just(Token::Comma),
        '[' => self.just(Token::LBracket),
        ']' => self.just(Token::RBracket),
        '{' => self.just(Token::LBrace),
        '}' => self.just(Token::RBrace),
        n if n.is_ascii_digit() => self.num(),
        _ => self.ident(),
      }
    } else {
      Token::Eof
    }
  }

  fn str(&mut self) -> Token {
    self.code.next();
    let s = self
      .code
      .by_ref()
      .take_while(|c| *c != '"')
      .collect::<String>();
    Token::Str(s)
  }

  fn num(&mut self) -> Token {
    todo!()
  }

  fn just(&mut self, t: Token) -> Token {
    self.code.next();
    t
  }

  fn ident(&mut self) -> Token {
    let mut s = String::new();
    while let Some(chr) = self.code.peek() {
      if chr.is_alphanumeric() {
        s.push(self.code.next().unwrap());
      } else {
        break;
      }
    }
    if &s == "false" {
      Token::False
    } else if &s == "true" {
      Token::True
    } else if &s == "null" {
      Token::Null
    } else {
      Token::IllegalIdent
    }
  }
}

pub struct Par<'json> {
  cur: Token,
  nxt: Token,
  lex: Lex<'json>,
  mem: Allocator<JsonValue>,
}

impl<'json> Par<'json> {
  fn init(mut lex: Lex<'json>, mem: usize) -> Self {
    let cur = lex.next_token();
    let nxt = lex.next_token();
    let mem = Allocator::make(mem);
    Self { cur, nxt, lex, mem }
  }

  fn advance(&mut self) -> Token {
    let mut ret = self.lex.next_token();
    std::mem::swap(&mut self.nxt, &mut self.cur);
    std::mem::swap(&mut self.nxt, &mut ret);
    ret
  }

  pub fn parse(
    src: &'json str,
    mem: usize,
  ) -> Result<(JsonValue, Allocator<JsonValue>), String> {
    let mut parser = Self::init(Lex::new(src), mem);
    let result = parser.go_parse()?;
    Ok((result, parser.mem))
  }

  pub fn go_parse(&mut self) -> Result<JsonValue, String> {
    let tk = match &mut self.cur {
      Token::False => Ok(JsonValue::Bool(false)),
      Token::True => Ok(JsonValue::Bool(true)),
      Token::Null => Ok(JsonValue::Null),
      Token::Str(s) => Ok(JsonValue::String(std::mem::take(s))),
      Token::Num(n) => Ok(JsonValue::Number(std::mem::take(n))),

      Token::LBracket => {
        let mut list = Vec::new();
        self.advance();
        loop {
          if matches!(self.cur, Token::RBracket) {
            break;
          }
          if matches!(self.cur, Token::Comma) {
            self.advance();
          }
          let e = self.go_parse()?;
          let id = self.mem.alloc(e);
          list.push(id);
        }
        Ok(JsonValue::List(list))
      }
      Token::RBracket => todo!(),

      Token::LBrace => {
        let mut obj = HashMap::new();
        self.advance();
        loop {
          if matches!(self.cur, Token::RBrace) {
            break;
          }
          if matches!(self.cur, Token::Comma) {
            self.advance();
          }
          let key = self.expect_str()?;
          if matches!(self.cur, Token::Colon) {
            self.advance();
          } else {
            return Err("Expected ':'.".to_string());
          }
          let val = self.go_parse()?;
          let id = self.mem.alloc(val);
          obj.insert(key, id);
        }
        Ok(JsonValue::Object(obj))
      }
      Token::RBrace => todo!(),

      Token::Comma => todo!(),
      Token::Colon => todo!(),

      Token::Eof => return Err("Reached EOF.".to_string()),
      Token::IllegalIdent => todo!(),
    };
    self.advance();
    tk
  }

  fn expect_str(&mut self) -> Result<String, String> {
    let s = match &mut self.cur {
      Token::Str(s) => std::mem::take(s),
      _ => return Err("Key is not a String".to_string()),
    };
    self.advance();
    Ok(s)
  }
}

fn main() {
  let src = include_str!("../test.json");

  match Par::parse(src, 1 << 4) {
    Ok((res, mem)) => {
      for el in mem.vec {
        println!("{el:?}");
      }
      println!("{res:?}");
    }
    Err(e) => eprintln!("{e}"),
  }
}

pub struct Allocator<T> {
  curr: usize,
  size: usize,
  vec: Vec<T>,
}

#[derive(Debug)]
pub struct Id<T>(usize, PhantomData<T>);

impl<T> Id<T> {
  pub fn id(id: usize) -> Self {
    Self(id, PhantomData)
  }
}

impl<T> Allocator<T> {
  pub fn make(size: usize) -> Self {
    assert!(size > 0);
    let vec = Vec::with_capacity(size - 1);
    Self {
      curr: 0,
      size: size - 1,
      vec,
    }
  }

  pub fn alloc(&mut self, el: T) -> Id<T> {
    let id = self.curr;
    assert!(id < self.size);
    self.vec.push(el);
    self.curr += 1;
    Id(id, PhantomData)
  }

  pub fn fetch(&self, Id(id, ..): Id<T>) -> &T {
    &self.vec[id]
  }
}
