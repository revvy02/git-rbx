-- (center CFrame, size) for a Model (via :GetBoundingBox()) or a BasePart.
-- An optional pivot folds the box back into the pivot's frame so framing is
-- measured relative to where the model is "held" rather than its raw bounds.
-- Ported from townies viewports/Utils/getTrueBoundingBox.
local function getBoundingBox(model: PVInstance, pivot: PVInstance?): (CFrame, Vector3)
	local cframe, size

	if model:IsA("Model") then
		cframe, size = model:GetBoundingBox()
	else
		cframe, size = (model :: BasePart).CFrame, (model :: BasePart).Size
	end

	if pivot and pivot ~= model then
		local pivotCF = pivot:GetPivot()
		local relativeCF = pivotCF:ToObjectSpace(cframe)
		cframe = cframe * CFrame.new(-relativeCF.Position) * relativeCF.Rotation:Inverse()
	end

	return cframe, size
end

return getBoundingBox
