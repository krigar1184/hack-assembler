#[macro_use]
extern crate lazy_static;
extern crate regex;

use std::path::Path;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::fmt;
use std::collections::HashMap;
use std::sync::Mutex;
use regex::{Regex, Captures};


lazy_static! {
    static ref SYMBOL_TABLE: Mutex<HashMap<String, usize>> = {
        let mut symbol_table = HashMap::new();

        for i in 0..16 {
            symbol_table.insert(format!("R{}", i), i);
        }

        symbol_table.insert(String::from("SCREEN"), 16384);
        symbol_table.insert(String::from("KBD"), 24576);
        symbol_table.insert(String::from("SP"), 0);
        symbol_table.insert(String::from("LCL"), 1);
        symbol_table.insert(String::from("ARG"), 2);
        symbol_table.insert(String::from("THIS"), 3);
        symbol_table.insert(String::from("THAT"), 4);

        Mutex::new(symbol_table)
    };

    static ref A_INSTRUCTION_REGEX: Regex = Regex::new(r"@([0-9]+)").unwrap();
    static ref C_INSTRUCTION_REGEX: Regex = Regex::new(r"^(\s+)?((?P<dest>[a-zA-Z]+)=)?(?P<comp>[a-zA-Z0-9\+\-!&|]+)(;(?P<jump>\w+))?(\s+//)?").unwrap();
    static ref LABEL_REGEX: Regex = Regex::new(r"\(([\$\.A-Za-z_0-9]+)\)").unwrap();
    static ref VARIABLE_REGEX: Regex = Regex::new(r"@([\$\.A-Za-z_]+([0-9]+)?)").unwrap();
}

static mut MEM_START: usize = 16;


trait Instruction {
    fn new(data: &str) -> Self;
    fn parse(&self) -> String;
}


struct AInstruction {
    value: String,
}


struct CInstruction {
    dest: String,
    comp: String,
    jump: String,
}


impl fmt::Display for AInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A Instruction: {}", self.value)
    }
}


impl Instruction for AInstruction {
    fn new(data: &str) -> Self {
        if VARIABLE_REGEX.is_match(data) {
            let captures: Captures = VARIABLE_REGEX.captures(&data).unwrap();
            let name = captures.get(1).unwrap().as_str();

            if SYMBOL_TABLE.lock().unwrap().contains_key(&name[..]) {
                return AInstruction {
                    value: SYMBOL_TABLE.lock().unwrap().get(&name[..]).unwrap().to_string(),
                };
            } else {
                unsafe {
                    let value = get_variable_value();
                    SYMBOL_TABLE.lock().unwrap().insert(String::from(name), value);
                    return AInstruction { value: format!("{}", value) };
                }
            }
        }

        let captures: Captures = A_INSTRUCTION_REGEX.captures(data).unwrap();
        let value: &str  = captures.get(1).unwrap().as_str();

        AInstruction { value: String::from(value) }
    }

    fn parse(&self) -> String {
        format!("0{:015b}", self.value.parse::<u32>().unwrap())
    }
}


impl fmt::Display for CInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "C Instruction: {} {} {}", self.dest, self.comp, self.jump)
    }
}


impl Instruction for CInstruction {
    fn new(data: &str) -> Self {
        let parts: Captures = C_INSTRUCTION_REGEX.captures(&data).unwrap();
        let comp: &str = parts.name("comp").unwrap().as_str();

        let dest = match parts.name("dest") {
            Some(v) => &v.as_str(),
            None => "",
        };

        let jump = match parts.name("jump") {
            Some(v) => &v.as_str(),
            None => "",
        };

        CInstruction { 
            dest: String::from(dest),
            comp: String::from(comp),
            jump: String::from(jump),
        }
    }

    fn parse(&self) -> String {
        let dest = match self.dest.as_str() {
            "" => "000",
            "M" => "001",
            "D" => "010",
            "MD" => "011",
            "A" => "100",
            "AM" => "101",
            "AD" => "110",
            "AMD" => "111",
            _ => panic!("Invalid dest: {}", self.dest),
        };

        let comp = match self.comp.as_str() {
            "0" => "0101010",
            "1" => "0111111",
            "-1" => "0111010",
            "D" => "0001100",
            "A" => "0110000",
            "!D" => "0001101",
            "!A" => "0110001",
            "-D" => "0001111",
            "-A" => "0110011",
            "D+1" => "0011111",
            "A+1" => "0110111",
            "D-1" => "0001110",
            "A-1" => "0110010",
            "D+A" => "0000010",
            "D-A" => "0010011",
            "A-D" => "0000111",
            "D&A" => "0000000",
            "D|A" => "0010101",
            "M" => "1110000",
            "!M" => "1110001",
            "-M" => "1110011",
            "M+1" => "1110111",
            "M-1" => "1110010",
            "D+M" => "1000010",
            "D-M" => "1010011",
            "M-D" => "1000111",
            "D&M" => "1000000",
            "D|M" => "1010101",
            _ => panic!("Invalid comp: {}", self.comp),
        };

        let jump = match self.jump.as_str() {
            "" => "000",
            "JGT" => "001",
            "JEQ" => "010",
            "JGE" => "011",
            "JLT" => "100",
            "JNE" => "101",
            "JLE" => "110",
            "JMP" => "111",
            _ => panic!("Invalid jump: {}", self.jump),
        };

        let result: String = format!("111{}{}{}", &comp, &dest, &jump);
        result
    }
}


fn main() {
    let filename: String = std::env::args().nth(1).unwrap();
    let path = Path::new(&filename);
    let file = File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|v| v.unwrap()).filter(|v| !(v.is_empty() || v.starts_with("//"))).collect();

    parse_symbols(&lines);
    let data = parse_instructions(&lines);

    let mut outfile = File::create(format!("./{}.hack", filename)).unwrap();
    outfile.write(&data.as_slice()).unwrap();
}


fn parse_symbols(lines: &Vec<String>) -> () {
    let mut counter = 0;

    for line in lines {
        if LABEL_REGEX.is_match(line) {
            let captures: Captures = LABEL_REGEX.captures(&line).unwrap();
            let key = String::from(captures.get(1).unwrap().as_str());
            let value = counter;

            SYMBOL_TABLE.lock().unwrap().insert(key, value);
        } else {
            counter += 1;
        }
    }
}


unsafe fn get_variable_value() -> usize {
    let value = MEM_START;
    MEM_START += 1;

    value
}


fn parse_instructions(lines: &Vec<String>) -> Vec<u8> {
    let mut data = Vec::new();

    for line in lines {
        if LABEL_REGEX.is_match(line) {
            continue;
        }

        let (a_instruction, c_instruction) = make_instructions(&line);

        if let Some(v) = a_instruction {
            data.extend(v.parse().bytes());
        }

        if let Some(v) = c_instruction {
            data.extend(v.parse().bytes());
        }

        data.extend(b"\n");

    }

    data
}


fn make_instructions(data: &str) -> (Option<AInstruction>, Option<CInstruction>) {
    if C_INSTRUCTION_REGEX.is_match(data) {
        return (None, Some(CInstruction::new(data)));
    } else if A_INSTRUCTION_REGEX.is_match(data) || VARIABLE_REGEX.is_match(data) {
        return (Some(AInstruction::new(data)), None);
    } else {
        return (None, None);
    }
}
