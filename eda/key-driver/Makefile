all: gerber pcb2gcode process_gcode

gerber: key-driver.kicad_pcb
	mkdir -p gerber
	cd gerber && \
	kicad-cli pcb export gerbers -l F.Cu,Edge.Cuts ../key-driver.kicad_pcb && \
	kicad-cli pcb export drill --format excellon ../key-driver.kicad_pcb && \
	cd ..

pcb2gcode: gerber/key-driver.drl \
		   gerber/key-driver-Edge_Cuts.gm1 \
		   gerber/key-driver-F_Cu.gtl \
		   gerber/key-driver-job.gbrjob
	mkdir -p gcode
	pcb2gcode \
		--drill gerber/key-driver.drl \
		--front gerber/key-driver-F_Cu.gtl \
		--outline gerber/key-driver-Edge_Cuts.gm1 \
		--basename key-driver
	rm gcode/*.svg

process_gcode: gcode/tracks.gcode gcode/drill.gcode gcode/outline.gcode

dx := -1.22495
dy := -1.22501
xm := 60.200019999999995

gcode/tracks.gcode: gerber/key-driver-F_Cu.gtl gerber/key-driver-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/key-driver_front.ngc \
	| gctk mirror -x $(xm) \
	| ../rotate-quarter-turn.sh > ./gcode/tracks.gcode
	rm ./gcode/key-driver_front.ngc

gcode/drill.gcode: gerber/key-driver.drl gerber/key-driver-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/key-driver_milldrill.ngc \
	| gctk mirror -x $(xm) \
	| ../rotate-quarter-turn.sh > ./gcode/drill.gcode
	rm ./gcode/key-driver_milldrill.ngc

gcode/outline.gcode: gerber/key-driver-Edge_Cuts.gm1 gerber/key-driver-job.gbrjob
	gctk translate -x $(dx) -y $(dy) < ./gcode/key-driver_outline.ngc \
	| gctk mirror -x $(xm) \
	| ../rotate-quarter-turn.sh > ./gcode/outline.gcode
	rm ./gcode/key-driver_outline.ngc

clean:
	rm -rf gerber
	rm -rf gcode
