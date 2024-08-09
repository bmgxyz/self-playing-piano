import cadquery as cq
from cadquery import exporters
import os

outer_r = 9.4
inner_r = 4.7625+0.5;
cap_t = 1.8
cap_inset_t = 0.9
wall_t = 0.6
wall_outer_r = inner_r + wall_t
solenoid_h = 32
press_fit_tol = 0.2
hole_pos = 21
hole_d = 3.5
taper_frac = 0.15
taper_mag = 0.4

solenoid_support = (
    cq.Workplane("XY")
    .rect(hole_pos*2+hole_d+3,(inner_r+wall_t)*2)
    .circle(inner_r)
    .pushPoints([(hole_pos,0),(-hole_pos,0)])
    .circle(hole_d/2)
    .extrude(cap_t)
    .faces(">Z")
    .workplane()
    .circle(inner_r)
    .circle(inner_r+wall_t)
    .extrude(solenoid_h*(1-taper_frac))
    .faces(">Z")
    .workplane()
    .transformed(rotate=(90,0,0))
    .moveTo(inner_r,0)
    .line(inner_r*taper_mag,solenoid_h*taper_frac)
    .line(wall_t,0)
    .line(-inner_r*taper_mag,-solenoid_h*taper_frac)
    .close()
    .revolve()
)

show_object(solenoid_support)
cd = os.path.dirname(os.path.abspath(__file__))
exporters.export(solenoid_support,f"{cd}/solenoid-support.stl")
