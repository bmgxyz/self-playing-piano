import cadquery as cq
from common import *

hole_pos = 21
hole_d = 3.1
spindle_d = 10
spindle_l = 75
spindle_tol = 0.5

spindle = (
    cq.Workplane("XY")
    .rect(60,15)
    .pushPoints([(hole_pos,0),(-hole_pos,0)])
    .circle(hole_d/2)
    .extrude(1.8)
    .circle(spindle_d/2)
    .extrude(spindle_l)
)

support = (
    cq.Workplane("XY")
    .rect(52,20)
    .extrude(2)
    .pushPoints([(-25,0),(25,0)])
    .eachpoint(
        lambda p: (
            cq.Workplane()
            .rect(2,20)
            .extrude(75)
            .faces(">X")
            .workplane()
            .moveTo(0,65)
            .hole(spindle_d + spindle_tol)
            .val()
            .located(p)
        ),combine=True
    )
)

export_stl(spindle, "winding-spindle")
export_stl(support, "winding-support")
