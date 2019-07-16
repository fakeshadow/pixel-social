import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/api/TalkAPI.dart';
import 'package:pixel_flutter/api/DataBase.dart';

import 'package:pixel_flutter/models/Talk.dart';

TalkRepo talkRepo = TalkRepo();

class TalkRepo {
  static final TalkRepo _talkRepo = TalkRepo._internal();

  factory TalkRepo() {
    return _talkRepo;
  }

  TalkRepo._internal();

  void init({String token}) {
    sockets.handleConn(() {
      if (token != null) {
        sendMessage('/auth ' + token);
      }
    });
  }

  void close() {
    sockets.close();
  }

  void addListener(Function callback) {
    sockets.addListener(callback);
  }

  void removeListener(Function callback) {
    sockets.removeListener(callback);
  }

  void sendMessage(String msg) {
    sockets.send(msg);
  }

  void setTalks({List<Talk> talks, Database db}) {
    DataBase.setTalks(talks: talks, db: db);
  }

  Future<List<Talk>> getTalksLocal({Database db}) async {
    return DataBase.getTalks(db: db);
  }
}
