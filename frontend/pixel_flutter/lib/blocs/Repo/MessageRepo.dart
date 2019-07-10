import 'package:pixel_flutter/models/Message.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../env.dart';

class MessageRepo with env {

  Future<void> saveMsg(
      {List<PublicMessage> pubMsg, List<PrivateMessage> prvMsg}) async {
    SharedPreferences prefs = await SharedPreferences.getInstance();

    pubMsg.forEach((PublicMessage msg) {
      final _id = msg.talkId;
      final _date = msg.dateTime;
      final String key = 'talk:$_id:$_date';
      prefs.setString(key, msg.message);
    });

    prvMsg.forEach((PrivateMessage msg) {
      final _id = msg.userId;
      final _date = msg.dateTime;
      final String key = 'user:$_id:$_date';
      prefs.setString(key, msg.message);
    });
  }
}
