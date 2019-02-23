A simple social app for PxielShare blockchain

Built with Flutter and Fastify


API endpoints


/api/user
{
	"type": "getUser",
	"uid": 5
}




/api/post/add
{
    "toPid": 1,
	"toTid": 1,
	"postContent": "1"

}

set uid and topid to 0 for getting posts for an topic;
set uid and toTid to 0 for getting reply posts for an post;

/api/post/get
{
	"type": "getPosts",
    "uid":0,
    "toPid": 0,
	"toTid" : 8,
	"page":1
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