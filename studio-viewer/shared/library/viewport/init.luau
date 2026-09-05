--[[
	Vendored camera-framing utils from game-prototype's miu-viewport
	(themselves ported from townies viewports/Utils): bounding-box measurement
	and FOV/aspect-fitted camera placement. Used by the resolver's View button
	to frame a conflict target whatever its size, instead of a fixed offset.
]]
return {
	getBoundingBox = require("@shared/library/viewport/getBoundingBox"),
	getIdealCameraDistance = require("@shared/library/viewport/getIdealCameraDistance"),
	getIdealCameraCFrame = require("@shared/library/viewport/getIdealCameraCFrame"),
	presets = require("@shared/library/viewport/presets"),
}
