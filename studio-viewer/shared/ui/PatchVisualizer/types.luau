--[[
	Neutral patch shape consumed by the PatchVisualizer, independent of where
	the changes came from (a conflict's per-side impact, or a two-file diff).

	`nodes` holds every instance that appears in the tree: changed instances
	AND the unchanged ancestors needed to reach them. An ancestor is any node
	with no `kind`. Children are discovered through `parent`; a node whose
	parent is absent from the map is a root.

	Values are Rust's serialized diff::PropertyValue (typed JSON) so the
	renderer can show swatches and stay truthful for properties Studio cannot
	read. A row may carry pre-formatted RichText instead (`currentText`).
]]

export type Id = number | string

export type PatchValue = any

export type ChangeKind = "Edit" | "Reparent" | "Pivot" | "Add"

export type ChangeRow = {
	property: string,
	kind: ChangeKind,
	current: PatchValue?,
	incoming: PatchValue?,
	currentText: string?, -- RichText override; wins over `current`
	incomingText: string?,
	warning: boolean?,
}

export type NodeKind = "Add" | "Remove" | "Edit" | "Reparent" | "Pivot"

export type PatchNodeInput = {
	id: Id,
	name: string,
	className: string,
	parent: Id?,
	instance: Instance?,
	kind: NodeKind?, -- nil for an unchanged ancestor; derived from rows when absent but rows exist
	rows: { ChangeRow }?,
	subtreeCount: number?, -- Remove: instances in the removed subtree
	warning: boolean?,
}

export type Patch = {
	nodes: { [Id]: PatchNodeInput },
}

export type PatchNode = {
	id: Id,
	name: string,
	className: string,
	instance: Instance?,
	kind: NodeKind?,
	rows: { ChangeRow },
	subtreeCount: number?,
	warning: boolean?,
	parent: PatchNode?,
	children: { PatchNode },
	depth: number,
	isFinalChild: boolean,
}

export type RowKind = "node" | "header" | "change"

export type Row = {
	y: number,
	h: number,
	kind: RowKind,
	index: number,
	node: PatchNode,
	change: ChangeRow?,
	isFinalElement: boolean,
	depthsComplete: { [number]: boolean },
}

return {}
