class Post {
  final int id, userId, topicId, postId, replyCount;
  final String postContent, createdAt;

  Post(
      {this.id,
      this.userId,
      this.topicId,
      this.postId,
      this.postContent,
      this.createdAt,
      this.replyCount});

  Post.fromJson(Map json)
      : id = json['id'],
        userId = json['user_id'],
        topicId = json['topic_id'],
        postId = json['post_id'],
        postContent = json['post_content'],
        createdAt = json['created_at'],
        replyCount = json['reply_count'];
}
