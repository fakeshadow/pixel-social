A simple social app for PxielShare blockchain

Built with Flutter and Fastify


API endpoints

/api/post/add
{
    "toPid": 1,
	"toTid": 1,
	"postContent": "1"

}


/api/topic/add
{
	"cid": "2",
	"topicContent": "abcdefghijklmn",
	"postContent": "ttttttttttt"
}

/api/topic/get
{
	"cids": ["1","2"],
	"page": 1
}