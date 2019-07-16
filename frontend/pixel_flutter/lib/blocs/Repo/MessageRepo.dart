import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/api/DataBase.dart';

import 'package:pixel_flutter/models/Message.dart';

MessageRepo categoryRepo = MessageRepo();

class MessageRepo {
  static final MessageRepo _categoryRepo = MessageRepo._internal();

  factory MessageRepo() {
    return _categoryRepo;
  }

  MessageRepo._internal();

  static Future<void> saveMsg({Database db, List<Message> msg}) async {
    return DataBase.setMsg(msg: msg, db: db);
  }

  static Future<List<Message>> loadMsg(
      {Database db, int time, int talkId, int userId}) async {
    return DataBase.getMsg(userId: userId, talkId: talkId, time: time, db: db);
  }
}
