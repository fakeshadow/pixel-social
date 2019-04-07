import 'package:equatable/equatable.dart';

class Topic extends Equatable {
  final int uid, tid, mainPid, postCount;
  final String username, cid, topicContent, lastPostTime, avatar;
  
  Topic({
    this.tid,
    this.cid,
    this.mainPid,
    this.topicContent,
    this.postCount,
    this.lastPostTime,
    this.username,
    this.uid,
    this.avatar
  }) : super([
          tid,
          cid,
          mainPid,
          topicContent,
          postCount,
          lastPostTime,
          username,
          avatar,
          uid
        ]);

  @override
  String toString() => 'Topic {tid: $tid}';
}
