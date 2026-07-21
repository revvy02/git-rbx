-- (center CFrame, size) for a Model, BasePart, or a container whose
-- descendants include BaseParts. Container bounds are world-axis-aligned and
-- include all eight corners of rotated parts, so Folder conflict targets can
-- be framed without widening the selection to their parent model.
-- An optional pivot folds the box back into the pivot's frame so framing is
-- measured relative to where the model is "held" rather than its raw bounds.
-- Ported from townies viewports/Utils/getTrueBoundingBox.
local function getBoundingBox(root: Instance, pivot: PVInstance?): (CFrame, Vector3)
	local cframe, size

	if root:IsA("Model") then
		cframe, size = root:GetBoundingBox()
	elseif root:IsA("BasePart") then
		cframe, size = root.CFrame, root.Size
	else
		local minimum = Vector3.new(math.huge, math.huge, math.huge)
		local maximum = Vector3.new(-math.huge, -math.huge, -math.huge)
		local partCount = 0
		for _, descendant in root:GetDescendants() do
			if descendant:IsA("BasePart") then
				partCount += 1
				local halfSize = descendant.Size / 2
				for x = -1, 1, 2 do
					for y = -1, 1, 2 do
						for z = -1, 1, 2 do
							local corner = descendant.CFrame:PointToWorldSpace(
								Vector3.new(x * halfSize.X, y * halfSize.Y, z * halfSize.Z)
							)
							minimum = Vector3.new(
								math.min(minimum.X, corner.X),
								math.min(minimum.Y, corner.Y),
								math.min(minimum.Z, corner.Z)
							)
							maximum = Vector3.new(
								math.max(maximum.X, corner.X),
								math.max(maximum.Y, corner.Y),
								math.max(maximum.Z, corner.Z)
							)
						end
					end
				end
			end
		end
		assert(partCount > 0, `{root:GetFullName()} has no spatial descendants`)
		cframe = CFrame.new((minimum + maximum) / 2)
		size = maximum - minimum
	end

	if pivot and pivot ~= root then
		local pivotCF = pivot:GetPivot()
		local relativeCF = pivotCF:ToObjectSpace(cframe)
		cframe = cframe * CFrame.new(-relativeCF.Position) * relativeCF.Rotation:Inverse()
	end

	return cframe, size
end

return getBoundingBox
