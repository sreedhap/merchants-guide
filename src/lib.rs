#![feature(slice_patterns)]

#[macro_use]
extern crate failure;

use failure::{Error, ResultExt};
use std::{collections::BTreeMap, io::{BufRead, BufReader, Read, Write}, str::FromStr};

fn roman_to_decimal(romans: impl Iterator<Item = Roman>) -> Result<u32, Error> {
    let mut decimal = 0;
    let mut iter = romans.peekable();
    while let Some(c) = iter.next() {
        let c: u32 = c.into();
        let mut multiplier = 1;
        if let Some(&n) = iter.peek() {
            let n: u32 = n.into();
            if n > c {
                multiplier = -1;
            }
        }
        decimal += i64::from(c) * multiplier;
    }
    if decimal == 0 {
        bail!("No romans literals provided")
    } else if decimal < 0 {
        bail!("Converted romans into negative decimal value {}", decimal);
    }
    Ok(decimal as u32)
}

#[derive(Debug, Default)]
struct ConversionTable {
    symbol_to_romans: BTreeMap<String, Roman>,
    product_prices: BTreeMap<String, f32>,
}

impl ConversionTable {
    fn symbols_to_romans(&self, values_space_separated: &str) -> Result<Vec<Roman>, Error> {
        values_space_separated
            .split_whitespace()
            .map(|s| {
                self.symbol_to_romans
                    .get(s)
                    .cloned()
                    .ok_or_else(|| format_err!("No roman value was associated with symbol '{}'", s))
            })
            .collect()
    }

    fn symbols_to_decimal(&self, symbol_space_separated: &str) -> Result<u32, Error> {
        roman_to_decimal(self.symbols_to_romans(symbol_space_separated)?.into_iter())
    }

    fn update(
        &mut self,
        tokens: impl Iterator<Item = Result<Token, Error>>,
        mut answer: impl FnMut(Query, &ConversionTable) -> Result<(), Error>,
    ) -> Result<(), Error> {
        use self::Token::*;
        for token in tokens {
            match token {
                Ok(token) => match token {
                    Other(query) => answer(query, self)?,
                    RomanNumeralMapping { symbol, roman } => {
                        self.symbol_to_romans.insert(symbol, roman);
                    }
                    PriceAssignment {
                        credits,
                        product,
                        symbols_space_separated,
                    } => {
                        let product_price =
                            credits / self.symbols_to_decimal(&symbols_space_separated)? as f32;
                        self.product_prices.insert(product, product_price);
                    }
                },
                Err(err) => return Err(err),
            }
        }
        Ok(())
    }
}

enum Query {
    Other,
    Roman {
        symbols_space_separated: String,
    },
    Product {
        symbols_space_separated: String,
        product: String,
    },
}

impl Query {
    fn answer(&self, table: &ConversionTable) -> Result<String, Error> {
        use self::Query::*;
        Ok(match self {
            Other => String::from("I have no idea what you are talking about"),
            Product {
                symbols_space_separated,
                product,
            } => {
                let single_product_price = table.product_prices.get(product).ok_or_else(|| {
                    format_err!("Product named '{}' was not yet encountered", product)
                })?;
                let decimal_multiplier = table.symbols_to_decimal(symbols_space_separated)?;
                let product_price = decimal_multiplier as f32 * single_product_price;
                format!(
                    "{} {} is {} Credits",
                    symbols_space_separated, product, product_price
                )
            }
            Roman {
                symbols_space_separated,
            } => {
                let decimal_value = table.symbols_to_decimal(symbols_space_separated)?;
                format!("{} is {}", symbols_space_separated, decimal_value)
            }
        })
    }
}

#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
enum Roman {
    I,
    V,
    X,
    L,
    C,
    D,
    M,
}

impl FromStr for Roman {
    type Err = Error;

    fn from_str(s: &str) -> Result<Roman, Error> {
        use self::Roman::*;
        Ok(match s {
            "I" => I,
            "V" => V,
            "X" => X,
            "L" => L,
            "C" => C,
            "D" => D,
            "M" => M,
            _ => return Err(format_err!("Invalid Roman numeral: '{}'", s)),
        })
    }
}

impl From<Roman> for u32 {
    fn from(this: Roman) -> u32 {
        use self::Roman::*;
        match this {
            I => 1,
            V => 5,
            X => 10,
            L => 50,
            C => 100,
            D => 500,
            M => 1000,
        }
    }
}

enum Token {
    RomanNumeralMapping {
        symbol: String,
        roman: Roman,
    },
    PriceAssignment {
        credits: f32,
        product: String,
        symbols_space_separated: String,
    },
    Other(Query),
}

impl FromStr for Token {
    type Err = Error;

    fn from_str(s: &str) -> Result<Token, Error> {
        use self::Token::*;
        let tokens: Vec<_> = s.split_whitespace().collect();
        Ok(match *tokens.as_slice() {
            [symbol, "is", roman] => RomanNumeralMapping {
                symbol: symbol.to_owned(),
                roman: roman.parse()?,
            },
            [ref symbols.., product, "is", credits, "Credits"] => PriceAssignment {
                credits: credits.parse::<f32>().with_context(|_| {
                    format!("Could not parse floating point number from '{}'", credits)
                })?,
                product: product.to_owned(),
                symbols_space_separated: symbols.join(" "),
            },
            ["how", "much", "is", ref symbols.., "?"] => Other(Query::Roman {
                symbols_space_separated: symbols.join(" "),
            }),
            ["how", "many", "Credits", "is", ref symbols.., product, "?"] => {
                Other(Query::Product {
                    symbols_space_separated: symbols.join(" "),
                    product: product.to_owned(),
                })
            }
            ["how", "much", _.., "?"] => return Ok(Other(Query::Other)),
            _ => {
                return Err(format_err!("'{}' could not be parsed", s));
            }
        })
    }
}

fn parse(input: impl Read) -> impl Iterator<Item = Result<Token, Error>> {
    let input = BufReader::new(input);
    input.lines().map(|r| {
        r.context("Failed to read at least one line from input")
            .map_err(Into::into)
            .and_then(|line| Token::from_str(&line))
    })
}

pub fn answers(input: impl Read, mut output: impl Write) -> Result<(), Error> {
    let do_answer = |query: Query, table: &ConversionTable| {
        writeln!(output, "{}", query.answer(table)?).map_err(Into::into)
    };

    ConversionTable::default().update(parse(input), do_answer)
}
