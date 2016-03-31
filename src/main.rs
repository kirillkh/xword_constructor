extern crate ndarray;
extern crate rand;
extern crate regex;

use std::str;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::cmp::max;

use regex::bytes::Regex;

use ndarray::OwnedArray;

use self::board::Board;
use self::common::{dim, Word, Orientation, Placement, PlacementId, Dim};
use self::constructor::Constructor;

mod board;
mod common;
mod constructor;

fn main() {
	let path = Path::new("board.dat");
    let display = path.display();

    // Open the path in read-only mode, returns `io::Result<File>`
    let mut file = match File::open(&path) {
        // The `description` method of `io::Error` returns a string that
        // describes the error
        Err(why) => panic!("couldn't open {}: {}", display,
                                                   Error::description(&why)),
        Ok(file) => file,
    };

    // Read the file contents into a string, returns `io::Result<usize>`
    let mut vec = vec![];
    match file.read_to_end(&mut vec) {
        Err(why) => panic!("couldn't read {}: {}", display,
                                                   Error::description(&why)),
        Ok(_) => ()
    }
    
	let re = Regex::new(r"^(?m)(\d{1,3})x(\d{1,3})$((?:^[_x]+$)+)^-----$((?:^[a-z]+$)+)").unwrap();
	let caps = re.captures(&vec).unwrap();
	
	let h = str::from_utf8(caps.at(1).unwrap()).unwrap().parse::<dim>().unwrap();
	let w = str::from_utf8(caps.at(2).unwrap()).unwrap().parse::<dim>().unwrap();
	let board_str = caps.at(3).unwrap();
	let dic_str = caps.at(4).unwrap();
	
	let mut board: OwnedArray<bool, Dim> = OwnedArray::default(Dim(h, w));
	let re = Regex::new(r"(?m)(^[_x]+$)").unwrap();
    for (i, cap) in re.captures_iter(board_str).enumerate() {
    	for (j, &c) in cap.at(1).unwrap().iter().enumerate() {
    		let (i, j) = (i as i8, j as i8);
    		match c {
    			b'_' => board[Dim(i, j)] = true,
    			b'x' => board[Dim(i, j)] = false,
    			_    => panic!("unexpected char: {}", c)
    		}
    	}
    }
    println!("{:?}", board);
    
    
    
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
    
    
    
    let moves = vec![];
	
	let seq = Constructor::new(10, 10).construct(&moves);
	println!("seq = {:?}", seq);
}
