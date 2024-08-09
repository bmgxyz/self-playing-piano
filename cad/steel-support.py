import cadquery as cq
from cadquery import exporters
import os

solenoid_diam = 22
solenoid_h = 32
steel_t = 2
steel_tol = 0.2
screw_diam = 3
screw_tol = 0.5
plunger_diam = 9.525
plunger_tol = 2
wall_t = 1.2
hole_sep = 42

h = solenoid_h + wall_t * 2 + steel_t + steel_tol
head_w = wall_t * 4 + (steel_t + steel_tol) * 2 + solenoid_diam
foot_w = hole_sep - head_w
steel_support = (
    cq.Workplane("XZ")
    # Back wall
    .rect(head_w,h,centered=(True,False))
    .moveTo(head_w/2,0)
    .rect(foot_w,wall_t,centered=False)
    .moveTo(-head_w/2,0)
    .rect(-foot_w,wall_t,centered=False)
    .extrude(wall_t)
    # Inner arch
    .faces("<Y")
    .workplane()
    .move(-solenoid_diam/2,0)
    .line(0,solenoid_h)
    .line(solenoid_diam,0)
    .line(0,-solenoid_h)
    .line(wall_t,0)
    .line(0,solenoid_h+wall_t)
    .line(-wall_t-solenoid_diam-wall_t,0)
    .line(0,-solenoid_h-wall_t)
    .close()
    # Outer arch and feet
    .workplane()
    .move(-head_w/2+wall_t,0)
    .line(0,h-wall_t)
    .line(head_w-2*wall_t,0)
    .line(0,-h+wall_t)
    .line(wall_t+foot_w,0)
    .line(0,wall_t)
    .line(-foot_w,0)
    .line(0,h-wall_t)
    .line(-head_w,0)
    .line(0,-h+wall_t)
    .line(-foot_w,0)
    .line(0,-wall_t)
    .close()
    .extrude(solenoid_diam)
    # Plunger hole
    .faces(">Z")
    .workplane()
    .moveTo(0,-(solenoid_diam/2+wall_t))
    .circle((plunger_diam+plunger_tol)/2)
    .cutThruAll()
    # Screw holes
    .faces("<Z")
    .workplane()
    .pushPoints([
        ((head_w+foot_w)/2,wall_t+solenoid_diam/2),
        (-(head_w+foot_w)/2,wall_t+solenoid_diam/2)
    ])
    .circle((screw_diam+screw_tol)/2)
    .cutThruAll()
    # Wire hole
    .faces(">Y")
    .workplane(invert=True)
    .tag("back")
    .moveTo(0,-wall_t*1.5)
    .rect(head_w,wall_t)
    .cutBlind(wall_t)
    .workplaneFromTagged("back")
    .moveTo(0,-wall_t*1.5)
    .rect(solenoid_diam+wall_t*2,wall_t)
    .cutBlind(wall_t*2)
)

show_object(steel_support)
cd = os.path.dirname(os.path.abspath(__file__))
exporters.export(steel_support,f"{cd}/steel-support.stl")
