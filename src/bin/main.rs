extern crate xword;
extern crate regex;
extern crate ndarray;
extern crate rand;
extern crate getopts;

use getopts::Options;
use std::env;

use std::str;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::bytes::Regex;

use ndarray::{OwnedArray, Axis};

use xword::{FixedGrid, Constructor, dim, Orientation, Placement, MatrixDim, LineDim, Problem};
use xword::util;

fn main() {
    if let Some(opts) = parse_opts() {
    	with_opts(opts)
    }
}


fn with_opts(opts: Opts) {
    let bytes = read_problem(&*opts.prob_file);
    
    let problem = parse_problem(bytes);
    
    let placements = gen_placements(&problem);
    
    let dic_str : Vec<_> = problem.dic.iter().map(|word| (word.id, String::from_utf8_lossy(&*word.str))).collect();
    println!("DIC={:?}", dic_str);
    
	let dim = problem.board.dim();
	let seq = Constructor::new(dim.0, dim.1, &problem.dic, &placements).construct();
//	println!("seq = {:?}", seq);
	
	for &or in Orientation::values() {
		println!("------- {:?} -------", or);
		let moves = seq.iter().cloned().filter(|place| place.orientation == or).collect();
		print_board(dim.0, dim.1, moves);
	}
}


fn parse_opts() -> Option<Opts> {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { Some(m) }
        Err(f) => {
        	println!("Error: {}\n", f.to_string());
        	print_usage(&program, &opts);
        	None 
        }
    };
    
    if let Some(matches) = matches {
    	if matches.opt_present("h") {
	        print_usage(&program, &opts);
	        return None;
	    }
	    
	    let prob_file = if !matches.free.is_empty() {
	        matches.free[0].clone()
	    } else {
	        "problem.xword".to_string()
	    };
	    
	    Some(Opts{ prob_file: prob_file })
    } else {
    	None
    }
}

fn print_usage(program: &str, opts: &Options) {
    let brief = format!("Usage: {} [options] [PROBLEM_FILE]", program);
    print!("{}", opts.usage(&brief));
}


fn print_board(h: dim, w: dim, seq: Vec<Placement>) {
	let mut rng = util::make_rng();
	let mut board : FixedGrid<&Placement> = FixedGrid::new(h, w, &mut *rng);
//			static mut board: Board<Move> = Board::new(self.h, self.w);

//	for mv in seq.iter() {
//		board.place(mv); 
//	}
	board.place_all(seq.iter().collect());
	
	board.print();
}

fn gen_placements(problem: &Problem) -> Vec<Placement> {
	let mut sorted = problem.dic.clone();
	sorted.sort_by(|a, b| a.len().cmp(&b.len()));
	
	let mut out_placements = vec![];
	
	let board = &problem.board;
	let mut placement_id = 0;
	for &orientation in Orientation::values().iter() {
		let axis = 1 - orientation as usize;
		for i in 0 .. board.dim()[axis] {
			let line = board.subview(Axis(axis), i as usize);
			let mut run_len = 0;
			for j in 0 .. *line.dim() {
				match line[LineDim(j)] {
					true => {
						run_len += 1;
						let placements: Vec<Placement> = sorted.iter()
							.cloned()
							.map(|word| 
								if word.len() <= run_len {
									placement_id += 1;
									let (y, x) = orientation.align(i, j + 1 - word.len());
									Some(Placement::new(placement_id-1, orientation, y, x, word))
								} else { None }
							)
							.fuse()
							.flat_map(|word_opt| word_opt)
							.collect();
						out_placements.extend(placements);
					},
					false => {
						run_len = 0
					},
//					_ 	 => panic!("impossible")
				}
			}
		}
	}
	
	out_placements
}

fn read_problem(file: &str) -> Vec<u8> {
	let path = Path::new(file);
    let display = path.display();

	// 1. read the file
    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        // The `description` method of `io::Error` returns a string that
        // describes the error
        Err(why) => panic!("couldn't open {}: {}", display,
                                                   Error::description(&why)),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut bytes : Vec<u8> = vec![];
    match file.read_to_end(&mut bytes) {
        Err(why) => panic!("couldn't read {}: {}", display,
                                                   Error::description(&why)),
        Ok(_) => ()
    }
    
    bytes
} 


fn parse_problem(bytes: Vec<u8>) -> Problem {
    let bytes = bytes.into_iter().map(|b| if b'A'<=b && b<=b'Z' {
            b'a' + b - b'A'
        } else {
            b
        }
    ).collect::<Vec<_>>();
	let re = Regex::new(r"^(?m)(\d{1,3})x(\d{1,3})[\r\n]+((?:[_#]+[\r\n]{1,2})+)-----((?:[\r\n]{1,2}[a-z' ,!-]+)+)[\r\n]*$").unwrap();
	let caps = re.captures(&bytes).unwrap();
	
	let h = str::from_utf8(caps.at(1).unwrap()).unwrap().parse::<dim>().unwrap();
	let w = str::from_utf8(caps.at(2).unwrap()).unwrap().parse::<dim>().unwrap();
	let board_str = caps.at(3).unwrap();
	
	let dic_str = caps.at(4).unwrap();
	let re = Regex::new(r"(?m)\n?(^.+)").unwrap();
	let mut dic : Vec<Vec<u8>> = vec![];
    for cap in re.captures_iter(dic_str) {
    	let cap = cap.at(1).unwrap();
        let word = cap.to_vec();
        dic.push(word);
    }
    
	let mut board: OwnedArray<bool, MatrixDim> = OwnedArray::default(MatrixDim(h, w));
	let re = Regex::new(r"(?m)\n?(^[_#]+)").unwrap();
    for (j, cap) in re.captures_iter(board_str).enumerate() {
    	for (i, &c) in cap.at(1).unwrap().iter().enumerate() {
    		let (i, j) = (i as dim, j as dim);
    		match c {
    			b'_' => board[MatrixDim(j, i)] = true,
    			b'#' => board[MatrixDim(j, i)] = false,
    			_    => panic!("unexpected char: {}", c)
    		}
    	}
    }
    
//	let (dic, dic_arena) = dic_arena::dic_arena(dic);
	
    Problem::new(dic, board)
}



struct Opts {
	prob_file: String
}



#[cfg(test)]
mod placement_tests {
	use xword::*;
	use ndarray::*;

	fn word(str: &'static [u8]) -> Vec<u8> {
        str.to_vec()
	}
	
	#[test]
	fn placement_ids_are_nat() {
		let grid: OwnedArray<bool, MatrixDim> = OwnedArray::default(MatrixDim(4, 3));
		let words = vec![word(b"ab"), word(b"bc"), word(b"cde"), word(b"cdef"), word(b"fedc"), word(b"fedcb")];
		let problem = Problem::new(words, grid);
		let places = super::gen_placements(&problem);
		
		for (i, place) in places.into_iter().enumerate() {
			assert_eq!(i, place.id.0 as usize)
		}
	}
}	
