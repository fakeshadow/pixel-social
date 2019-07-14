import 'package:pixel_flutter/models/Message.dart';

import '../../env.dart';

class MessageRepo {

  Future<void> saveMsg(
      {List<PublicMessage> pubMsg, List<PrivateMessage> prvMsg}) async {

    pubMsg.forEach((PublicMessage msg) {
      final _id = msg.talkId;
      final _date = msg.dateTime;
      final String key = 'talk:$_id:$_date';
    });

    prvMsg.forEach((PrivateMessage msg) {
      final _id = msg.userId;
      final _date = msg.dateTime;
      final String key = 'user:$_id:$_date';
    });
  }
}
