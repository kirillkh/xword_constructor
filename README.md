# xword_constructor
Constructs crosswords using the NRPA algorithm (Monte Carlo Tree Search): http://www.chrisrosin.com/rosin-ijcai11.pdf

Crossword problem is read from the problem.xword file in the current directory. Most customization parameters are currently 
hardcoded as constants in constructor.rs. Currently lacks time limit functionality (will perform NRPA_ITERS iterations at each
one of NRPA_LEVEL levels of recursion).
