all: transition-diagram.svg

transition-diagram.svg: transition-diagram.gv
	dot transition-diagram.gv -Tsvg > transition-diagram.svg

clean:
	cargo clean
	rm -f *.svg
