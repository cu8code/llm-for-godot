extends VectorDB

var data := {
	"nami":"the book of love",
	"main":"the book of fire",
}

func _ready() -> void:
	var data : PackedInt32Array = [1,2,3,4]
	create_embeddings("github is used for code")
	create_embeddings("mac is a os")
	create_embeddings("linux is fun")
	match_item("linux")
