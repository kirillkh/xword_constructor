# xword_constructor
Constructs crosswords using the NRPA algorithm (Monte Carlo Tree Search): http://www.chrisrosin.com/rosin-ijcai11.pdf

Running instructions:

1. install Rust and Cargo
2. `git clone https://github.com/kirillkh/xword_constructor.git`
3. `cd xword_constructor`
4. `cargo run probgen` to generate a problem
5. `./run_xword.sh <PROBLEM FILE>` (or `cargo run --release --bin xword <PROBLEM FILE>` if you're on Windows) to solve the given problem

The project builds two binaries: probgen and xword. 

**xword** accepts a problem file as its input (see problem.xword for example). A problem file specifies its grid shape and the dictionary.
The grid shape contains two kinds of characters: "_" means "empty cell", "#" means "blocked cell". The constructor's job is to produce 
a valid crossword by placing as many words from the dictionary into the empty cells as possible. The constructor will print intermediate results to stdout once in a while and, after a set number of iterations (currently hardcoded), it will output the final result as two grids: one for horizontal and another for vertical words.

**probgen** generates problems. To generate a problem, run probgen without parameters. The output will be written into out_problem.xword file. 
To customize the template, edit the template.xtempl file. There are 3 characters that you can put in every cell of the template grid:
- "_" means "generate an empty cell with a random character"
- "#" means "generate a blocked cell"
- "*" means "generate an empty cell wihout a character"

Currently we only support very small dictionaries (hundreds of words).


Implementation notes:

- Most customization parameters are currently hardcoded as constants in constructor.rs. 
- Lacks time limit functionality (will perform NRPA_ITERS iterations at each one of NRPA_LEVEL levels of recursion).
