# xword_constructor
Constructs crosswords using the NRPA algorithm (Monte Carlo Tree Search): http://www.chrisrosin.com/rosin-ijcai11.pdf

Crossword problem is read from the problem.xword file in the current directory. Most customization parameters are currently 
hardcoded as constants in constructor.rs. Currently lacks time limit functionality (will perform NRPA_ITERS iterations at each
one of NRPA_LEVEL levels of recursion).

The project builds two binaries: probgen and xword. 

**xword** accepts a problem file as its input (see problem.xword for example). A problem file specifies its grid shape and the dictionary.
The grid shape contains two kinds of characters: "_" means "empty cell", "#" means "blocked cell". The constructor's job is to produce 
a valid crossword by placing as many words from the dictionary into the empty cells as possible. 

**probgen** generates problems. To generate a problem, run probgen without parameters. The output will be written into out_problem.xword file. 
To customize the template, edit the template.xtempl file. There are 3 characters that you can put in every cell of the template grid:
- "_" means "generate an empty cell with a random character"
- "#" means "generate a blocked cell"
- "*" means "generate an empty cell wihout a character"
