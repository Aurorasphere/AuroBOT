use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

impl Op {
    fn precedence(self) -> u8 {
        match self {
            Op::Add | Op::Sub => 1,
            Op::Mul | Op::Div => 2,
            Op::Pow => 3,
        }
    }

    fn is_right_associative(self) -> bool {
        matches!(self, Op::Pow)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Op(Op),
    LParen,
    RParen,
    Ident(String),
    Func(String),
}

#[derive(Debug)]
struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut chars = input.chars().peekable();
    let mut expect_unary = true;

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch.is_ascii_digit() || ch == '.' {
            let mut s = String::new();
            let mut dot_seen = ch == '.';
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() {
                    s.push(c);
                    chars.next();
                } else if c == '.' {
                    if dot_seen { break; }
                    dot_seen = true;
                    s.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            let num = s.parse::<f64>().map_err(|_| ParseError("잘못된 숫자 형식".to_string()))?;
            tokens.push(Token::Number(num));
            expect_unary = false;
            continue;
        }

        // function or identifier
        if ch.is_ascii_alphabetic() {
            let mut name = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    name.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(Token::Ident(name));
            expect_unary = true;
            continue;
        }

        match ch {
            '+' => {
                chars.next();
                tokens.push(Token::Op(Op::Add));
                expect_unary = true;
            }
            '-' => {
                chars.next();
                if expect_unary {
                    // unary minus: treat as 0 - x
                    tokens.push(Token::Number(0.0));
                    tokens.push(Token::Op(Op::Sub));
                } else {
                    tokens.push(Token::Op(Op::Sub));
                    expect_unary = true;
                }
            }
            '*' => {
                chars.next();
                tokens.push(Token::Op(Op::Mul));
                expect_unary = true;
            }
            '/' => {
                chars.next();
                tokens.push(Token::Op(Op::Div));
                expect_unary = true;
            }
            '^' => {
                chars.next();
                tokens.push(Token::Op(Op::Pow));
                expect_unary = true;
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
                expect_unary = true;
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
                expect_unary = false;
            }
            _ => {
                return Err(ParseError(format!("알 수 없는 문자: {}", ch)));
            }
        }
    }

    Ok(tokens)
}

fn to_rpn(tokens: &[Token]) -> Result<Vec<Token>, ParseError> {
    let mut output: Vec<Token> = Vec::new();
    let mut ops: Vec<Token> = Vec::new();

    for token in tokens.iter().cloned() {
        match token {
            Token::Number(_) => output.push(token),
            Token::Ident(_) => ops.push(token),
            Token::Func(name) => {
                // 함수 토큰이 입력에 직접 등장할 일은 없지만, 안전하게 출력으로 전달
                output.push(Token::Func(name));
            }
            Token::Op(op1) => {
                while let Some(Token::Op(op2)) = ops.last().cloned() {
                    if (op1.precedence() < op2.precedence())
                        || (op1.precedence() == op2.precedence() && !op1.is_right_associative())
                    {
                        output.push(ops.pop().unwrap());
                    } else {
                        break;
                    }
                }
                ops.push(Token::Op(op1));
            }
            Token::LParen => ops.push(Token::LParen),
            Token::RParen => {
                while let Some(top) = ops.pop() {
                    if let Token::LParen = top {
                        // If there is a function token on top, move to output
                        if let Some(Token::Ident(name)) = ops.last().cloned() {
                            ops.pop();
                            output.push(Token::Func(name));
                        }
                        break;
                    } else {
                        output.push(top);
                    }
                }
            }
        }
    }

    while let Some(top) = ops.pop() {
        match top {
            Token::LParen | Token::RParen => return Err(ParseError("괄호가 올바르지 않습니다".to_string())),
            _ => output.push(top),
        }
    }

    Ok(output)
}

fn eval_rpn(rpn: &[Token]) -> Result<f64, ParseError> {
    let mut stack: Vec<f64> = Vec::new();
    for token in rpn.iter().cloned() {
        match token {
            Token::Number(n) => stack.push(n),
            Token::Func(name) => {
                let x = stack.pop().ok_or_else(|| ParseError("피연산자가 부족합니다".to_string()))?;
                let v = match name.as_str() {
                    "sqrt" => {
                        if x < 0.0 { return Err(ParseError("sqrt의 입력은 음수가 될 수 없습니다".to_string())); }
                        x.sqrt()
                    }
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "tan" => x.tan(),
                    _ => return Err(ParseError(format!("알 수 없는 함수: {}", name))),
                };
                if v.is_finite() { stack.push(v); } else { return Err(ParseError("유효하지 않은 결과".to_string())); }
            }
            Token::Op(op) => {
                let b = stack.pop().ok_or_else(|| ParseError("피연산자가 부족합니다".to_string()))?;
                let a = stack.pop().ok_or_else(|| ParseError("피연산자가 부족합니다".to_string()))?;
                let v = match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => {
                        if b == 0.0 {
                            return Err(ParseError("0으로 나눌 수 없습니다".to_string()));
                        }
                        a / b
                    }
                    Op::Pow => a.powf(b),
                };
                stack.push(v);
            }
            Token::LParen | Token::RParen => {
                return Err(ParseError("RPN 단계에서 잘못된 토큰".to_string()));
            }
            Token::Ident(_) => unreachable!("Ident는 변환 단계에서만 사용됩니다"),
        }
    }
    if stack.len() != 1 {
        return Err(ParseError("표현식이 올바르지 않습니다".to_string()));
    }
    Ok(stack[0])
}

pub fn evaluate(expression: &str) -> Result<String, String> {
    let tokens = tokenize(expression).map_err(|e| e.to_string())?;
    let rpn = to_rpn(&tokens).map_err(|e| e.to_string())?;
    let v = eval_rpn(&rpn).map_err(|e| e.to_string())?;
    Ok(format_float(v))
}

fn format_float(v: f64) -> String {
    if v == 0.0 { return "0".to_string(); }
    let s = format!("{:.12}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s == "-0" { "0".to_string() } else { s }
}


