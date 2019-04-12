import 'package:equatable/equatable.dart';

class Post extends Equatable {
  final int id, userId, topicId, postId, replyCount;
  final String username, avatarUrl, postContent, lastReplyTime;

  Post(
      {this.id,
      this.userId,
      this.username,
      this.avatarUrl,
      this.topicId,
      this.postId,
      this.postContent,
      this.lastReplyTime,
      this.replyCount})
      : super([
          id,
          userId,
          username,
          avatarUrl,
          topicId,
          postId,
          postContent,
          lastReplyTime,
          replyCount
        ]);
}
