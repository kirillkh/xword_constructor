extern crate xword;
extern crate regex;
extern crate ndarray;
extern crate rand;

use std::str;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use regex::bytes::Regex;

use ndarray::{OwnedArray, Axis};

use xword::{Board, Constructor, dim, Word, WordId, Orientation, Placement, MatrixDim, LineDim, Problem};
use xword::util;

fn main() {
    
    let bytes = read_problem();
    
    let problem = parse(bytes);
    
    let placements = gen_placements(&problem);
    
    let dic_str : Vec<_> = problem.dic.iter().map(|word| (word.id, String::from_utf8_lossy(&*word.str))).collect();
    println!("DIC={:?}", dic_str);
//    println!("PLACEMENTS: {:?}", placements);
    
    
    
//    let mut w = 0;
//    let mut h = 0;
//    for (i, &b) in data.iter().enumerate() {
//    	match b {
//    		b'\n' => {
//    			let line = str::from_utf8(&data[0..i]).unwrap();
//    			
//    			let re = Regex::new(r"(\d{1,3})x(\d{1,3})").unwrap();
//				for cap in re.captures_iter(line) {
//					h = cap.at(1).unwrap().parse::<i32>().unwrap();
//					w = cap.at(2).unwrap().parse::<i32>().unwrap();
//				}
//    			
//    			data = &data[i+1..];
//    		},
//    		_ => {}
//    	}
//    }
//    
//    
//    
//    
//    let mut line_start = 0;
//    let mut j = 0;
//    for (i, &b) in data.into_iter().enumerate().skip(line_start) {
//    	match b {
//    		b'\n' => {
////    			line_start = i+1;
//				j += 1;
//    		},
//    		b'_' =>  {
//    			
//    		},
//    		b'x' =>  {
//    			
//    		},
//    		_    =>  {
//    			panic!("unrecornized symbol: {}", b);
//    		}
//    	}
//    }
    
    
    
	let dim = problem.board.dim();
	let seq = Constructor::new(dim.1, dim.0).construct(&placements);
//	println!("seq = {:?}", seq);
	
	for &or in Orientation::values() {
		println!("------- {:?} -------", or);
		let moves = seq.iter().cloned().filter(|place| place.orientation == or).collect();
		print_board(dim.1, dim.0, moves);
	}
}


fn print_board(h: dim, w: dim, seq: Vec<Placement>) {
	let mut rng = util::make_rng();
	let mut board : Board<&Placement> = Board::new(h, w, &mut *rng);
//			static mut board: Board<Move> = Board::new(self.h, self.w);

//	for mv in seq.iter() {
//		board.place(mv); 
//	}
	board.place(seq.iter().collect());
	
	board.print();
}

fn gen_placements(problem: &Problem) -> Vec<Placement> {
	let mut sorted = problem.dic.clone();
	sorted.sort_by(|a, b| a.len().cmp(&b.len()));
	
	let mut out_placements = vec![];
	
	let board = &problem.board;
	let mut placement_id = 0;
	for &orientation in Orientation::values().iter() {
		let axis = orientation as usize;
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
									let (x, y) = orientation.align(j + 1 - word.len(), i);
									Some(Placement::new(placement_id, orientation, x, y, word))
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

fn read_problem() -> Vec<u8> {
	let path = Path::new("problem.xword");
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


fn parse(bytes: Vec<u8>) -> Problem {
	let re = Regex::new(r"^(?m)(\d{1,3})x(\d{1,3})[\r\n]+((?:[_#]+[\r\n]{1,2})+)-----((?:[\r\n]{1,2}[a-z]+)+)[\r\n]*$").unwrap();
	let caps = re.captures(&bytes).unwrap();
	
	let h = str::from_utf8(caps.at(1).unwrap()).unwrap().parse::<dim>().unwrap();
	let w = str::from_utf8(caps.at(2).unwrap()).unwrap().parse::<dim>().unwrap();
	let board_str = caps.at(3).unwrap();
	
	let dic_str = caps.at(4).unwrap();
	let re = Regex::new(r"(?m)\n?(^.+)").unwrap();
	let mut dic : Vec<Word> = vec![];
	let mut id : WordId = 0;
    for cap in re.captures_iter(dic_str) {
    	let cap = cap.at(1).unwrap();
    	let boxed = cap.to_vec().into_boxed_slice();
    	dic.push(Word::new(id, boxed));
    	id += 1;
    }
	
	let mut board: OwnedArray<bool, MatrixDim> = OwnedArray::default(MatrixDim(w, h));
	let re = Regex::new(r"(?m)\n?(^[_#]+)").unwrap();
    for (i, cap) in re.captures_iter(board_str).enumerate() {
    	for (j, &c) in cap.at(1).unwrap().iter().enumerate() {
    		let (i, j) = (i as dim, j as dim);
    		match c {
    			b'_' => board[MatrixDim(j, i)] = true,
    			b'#' => board[MatrixDim(j, i)] = false,
    			_    => panic!("unexpected char: {}", c)
    		}
    	}
    }
    
    Problem::new(dic, board)
}
