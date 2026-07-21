local getIdealCameraDistance = require("@self/getIdealCameraDistance")
local presets = require("@self/presets")

type Options = {
	fov: number?, -- field of view in degrees (default 70)
	scale: number?, -- multiplies the framing distance (default 1)
	preset: string?, -- key into `presets` (default "default")
	direction: Vector3?, -- overrides the preset direction
	aspectRatio: number?, -- viewport aspect (default 1)
	offset: Vector3?, -- added to the look-at target (default 0)
	rotation: number?, -- extra yaw in degrees around the model
}

-- Camera CFrame that frames a model whose bounds are (modelCF, modelSize).
-- Ported from townies viewports/Utils/getIdealCameraViewCFrame (options type
-- added for this strict-mode vendor).
local function getIdealCameraCFrame(modelCF: CFrame, modelSize: Vector3, optionsIn: Options?): CFrame
	local options: Options = optionsIn or {}

	local fov = options.fov or 70
	local scale = options.scale or 1
	local preset = options.preset or "default"
	local direction = options.direction or presets[preset] or presets.default
	local aspectRatio = options.aspectRatio or 1
	local offset = options.offset or Vector3.zero

	local distance = getIdealCameraDistance(fov, modelSize, aspectRatio)
	local scaledDistance = distance * scale
	local position = modelCF.Position + offset - direction.Unit * scaledDistance

	if options.rotation then
		local angle = math.rad(options.rotation)
		local rotatedDir = CFrame.Angles(0, angle, 0) * direction
		position = modelCF.Position + offset - rotatedDir.Unit * scaledDistance
	end

	return CFrame.lookAt(position, modelCF.Position + offset)
end

return getIdealCameraCFrame
