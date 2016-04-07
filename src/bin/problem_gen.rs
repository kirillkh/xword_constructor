extern crate xword;
extern crate regex;
extern crate ndarray;
extern crate rand;

use std::str;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::path::Path;

use regex::bytes::Regex;
use ndarray::{OwnedArray, Axis, ArrayView};
use rand::distributions::Range;

use xword::{dim, MatrixDim, LineDim, Problem, Orientation, Word};
use xword::util::{make_rng, AbstractRng};

fn main() {
    let bytes = read_template();
    
    let mut templ: OwnedArray<u8, MatrixDim> = parse(bytes);
    
    fill(&mut templ);
    
    print(&templ);
    
    let problem = gen_problem(&mut templ);
    
    write_problem(&problem);
}

fn print(filled: &OwnedArray<u8, MatrixDim>) {
	for (MatrixDim(y, x), c) in filled.indexed_iter() {
		if x==0 {
			print!("\n");
		}
		print!("{}", *c as char);
	}
	println!("");
}

fn fill(templ: &mut OwnedArray<u8, MatrixDim>) {
	let mut rng = make_rng();
	let range = Range::new(b'a', b'z');
	
	for j in 0..templ.dim()[1] {
		for i in 0..templ.dim()[0] {
			let idx = MatrixDim(i, j);
			match templ[idx] {
				b'_' => templ[idx] = rng.gen_u8(range),
				b'#' | b'*' => (),
				_	=> panic!("should never happen")
			}
		}
	}
}

fn gen_problem(templ: &mut OwnedArray<u8, MatrixDim>) -> Problem {
	let w = templ.dim()[0];
	let h = templ.dim()[1];
	
	let mut board: OwnedArray<bool, MatrixDim> = OwnedArray::default(MatrixDim(w, h));
	
	let mut wid = 0;
	let mut dic : Vec<Word> = vec![];
	
	let mut add_word = |dic: &mut Vec<_>, line: &ArrayView<_, _>, from, to| {
		if to - from > 1 {
			let v : Vec<u8> = line.iter()
				.skip(from)
				.take(to-from)
				.cloned()
				.collect();
			let word = Word::new(wid, v.into_boxed_slice());
			dic.push(word);
		}
		
		wid += 1;
	};
	
	for &orientation in Orientation::values().iter() {
		let axis = 1-orientation as usize;
		for i in 0 .. templ.dim()[axis] {
			let line = templ.subview(Axis(axis), i as usize);
			let mut run_start = 0;
			for j in 0 .. *line.dim() {
				match line[LineDim(j)] {
					b'#'|b'*' => {
						add_word(&mut dic, &line, run_start, j);
						run_start = j+1;
					},
					_ => {
					}
				}
			}
			
			add_word(&mut dic, &line, run_start, *line.dim());
		}
	}
	
	for i in 0 .. templ.dim()[0] {
		for j in 0 .. templ.dim()[1] {
			let idx = MatrixDim(i, j);
			match templ[idx] {
				b'#' => board[idx] = false,
				b'*' | _ => board[idx] = true
			}
		}
	}
	
	Problem::new(dic, board)
}

fn write_problem(problem: &Problem) {
	let path = Path::new("out_problem.xword");
    let display = path.display();
    
    let mut options = OpenOptions::new();
	options.create(true).write(true).truncate(true);


    let mut file = match options.open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display,
                                                   Error::description(&why)),
        Ok(file) => file,
    };
	
	let mut content = vec![];
	let dim = problem.board.dim();
	let dim_str = format!("{}x{}", dim[0], dim[1]);
	content.extend(dim_str.as_bytes());
	
	for (MatrixDim(y, x), &c) in problem.board.indexed_iter() {
//		println!("x={}, y={}", x, y);
		if x==0 {
			content.push(b'\n');
		}
		match c {
			true => content.push(b'_'),
			false => content.push(b'#'),
		}
	}
	
	content.extend(b"\n-----\n");
	
	for word in problem.dic.iter() {
		let str : &[u8] = &*word.str;
		content.extend(str);
		content.push(b'\n');
	}
	
	match file.write_all(&content) {
		Err(why) => panic!("couldn't write {}: {}", display, Error::description(&why)),
		Ok(()) => (),
	}
}

fn read_template() -> Vec<u8> {
	let path = Path::new("template.xtempl");
    let display = path.display();

    let mut file = match File::open(&path) {
        Err(why) => panic!("couldn't open {}: {}", display,
                                                   Error::description(&why)),
        Ok(file) => file,
    };

    let mut bytes : Vec<u8> = vec![];
    match file.read_to_end(&mut bytes) {
        Err(why) => panic!("couldn't read {}: {}", display,
                                                   Error::description(&why)),
        Ok(_) => ()
    }
    
    bytes
}

fn parse(bytes: Vec<u8>) -> OwnedArray<u8, MatrixDim> {
	let re = Regex::new(r"^(?m)(\d{1,3})x(\d{1,3})[\r\n]{1,2}((?:[_#\*]+[\r\n]{1,2})+)[\r\n]*$").unwrap();
	let caps = re.captures(&bytes).unwrap();
	
	let h = str::from_utf8(caps.at(1).unwrap()).unwrap().parse::<dim>().unwrap();
	let w = str::from_utf8(caps.at(2).unwrap()).unwrap().parse::<dim>().unwrap();
	let board_str = caps.at(3).unwrap();
	
	let mut board: OwnedArray<u8, MatrixDim> = OwnedArray::default(MatrixDim(h, w));
	let re = Regex::new(r"(?m)\n?(^[_#\*]+)").unwrap();
    for (i, cap) in re.captures_iter(board_str).enumerate() {
    	for (j, &c) in cap.at(1).unwrap().iter().enumerate() {
    		let (i, j) = (i as dim, j as dim);
    		match c {
    			b'_' | b'#' | b'*' => board[MatrixDim(i, j)] = c,
    			_    => panic!("unexpected char: {}", c)
    		}
    	}
    }
    
    board
}
