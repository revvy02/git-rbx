--[[
	Locate the stamped state container a Studio front end boots from.

	Places open in Studio directly, so the container is already in the DOM.
	Model files cannot be opened as a place: the rbxm is imported into a
	preview folder under Workspace instead. The disk file stays authoritative
	either way — the import is only ever eyes and highlights.
]]

local container = {}

container.PREVIEW_NAME = "GitRbxModelPreview"

function container.find(file: string, name: string): Instance?
	local found = game:FindFirstChild(name, true)
	if found or not file:match("%.rbxmx?$") then
		return found
	end
	local roblox = require("@rodeo/roblox")
	local stale = workspace:FindFirstChild(container.PREVIEW_NAME)
	if stale then
		stale:Destroy()
	end
	local preview = Instance.new("Folder")
	preview.Name = container.PREVIEW_NAME
	for _, root in roblox.import(file) do
		root.Parent = preview
	end
	preview.Parent = workspace
	return game:FindFirstChild(name, true)
end

return container
