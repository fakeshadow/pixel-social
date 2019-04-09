import 'package:equatable/equatable.dart';

class Topic extends Equatable {
  final int id, categoryId, userId, replyCount;
  final String username, title, body, thumbnail, lastReplyTime, avatarUrl;

  Topic(
      {this.id,
      this.categoryId,
      this.userId,
      this.replyCount,
      this.username,
      this.title,
      this.body,
      this.thumbnail,
      this.lastReplyTime,
      this.avatarUrl})
      : super([
          id,
          categoryId,
          userId,
          replyCount,
          username,
          title,
          body,
          thumbnail,
          lastReplyTime,
          avatarUrl
        ]);

  @override
  String toString() => 'Topic {id: $id}';
}
