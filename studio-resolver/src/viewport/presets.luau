-- Camera direction presets: the unit-ish vector pointing FROM the model TOWARD
-- the camera. Ported from townies viewports/Utils/getIdealCameraViewCFrame.
return {
	default = vector.create(1, -0.5, 1),
	front = vector.create(0, 0, 1),
	side = vector.create(1, 0, 0),
	top = vector.create(0, -1, 0),
	isometric = vector.create(1, -1, 1),
	heroic = vector.create(0.3, 0.5, 1), -- low angle, flatters characters
}
