class Post {
  final int uid, pid, toPid;
  final String postData, createdAt;

  Post({
    this.uid,
    this.pid,
    this.toPid,
    this.postData,
    this.createdAt,
  });

  Post.fromJson(Map json)
      : uid = json['uid'],
        pid = json['pid'],
        toPid = json['toPid'],
        postData = json['postData'],
        createdAt = json['createdAt'];
}
