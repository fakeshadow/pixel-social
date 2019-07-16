import 'package:equatable/equatable.dart';

class Message extends Equatable {
  final int talkId, userId;
  final DateTime dateTime;
  final String msg;

  Message({this.talkId, this.userId, this.dateTime, this.msg})
      : super([talkId, userId, dateTime, msg]);

  Map<String, dynamic> toMap() {
    return <String, dynamic>{
      'talk_id': talkId,
      'user_id': userId,
      'time': dateTime.millisecondsSinceEpoch,
      'msg': msg,
    };
  }
}
