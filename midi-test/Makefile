all: velocity-ramp.mid \
     pitch-ramp.mid \
     speed-ramp.mid

velocity-ramp.mid: velocity-ramp.py
	python3 velocity-ramp.py

pitch-ramp.mid: pitch-ramp.py
	python3 pitch-ramp.py

speed-ramp.mid: speed-ramp.py
	python3 speed-ramp.py

clean:
	rm *.mid
