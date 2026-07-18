-- Distance a camera needs in order to fully frame `targetSize` at the given
-- `fov` (degrees) and viewport aspect ratio. Considers horizontal, vertical and
-- depth extents, then adds 10% padding.
-- Ported from townies viewports/Utils/getIdealCameraDistanceToContainModel.
local function getIdealCameraDistance(fov: number, targetSize: Vector3, aspectRatio: number?): number
	aspectRatio = aspectRatio or 16 / 9

	local halfFOV = math.rad(fov) / 2
	local x, y, z = targetSize.X, targetSize.Y, targetSize.Z

	local horizontalDist = (x / 2) / math.tan(halfFOV)
	local verticalDist = (y / 2) / math.tan(halfFOV / aspectRatio)
	local depthDist = z / 2

	local maxDist = math.max(horizontalDist, verticalDist) + depthDist
	return maxDist * 1.1 -- 10% padding for safety
end

return getIdealCameraDistance
