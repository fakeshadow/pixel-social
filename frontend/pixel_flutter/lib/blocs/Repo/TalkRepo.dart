import 'package:sqflite/sqlite_api.dart';

import 'package:pixel_flutter/api/TalkAPI.dart';
import 'package:pixel_flutter/api/DataBase.dart';

import 'package:pixel_flutter/models/Talk.dart';
import 'package:pixel_flutter/models/User.dart';

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

  Future<List<Talk>> getTalks({Database db}) async {
    final List<Talk> users = await DataBase.getTalks(db: db).catchError((e) {
      return null;
    });
    if (users == null) {
      sendMessage(SendMsg(cmd: CommandType.getTalks, talkId: 0).toJSON());
    }
    return users;
  }

  Future<List<User>> getRelation({Database db}) async {
    final List<User> users = await DataBase.getUsers(db: db).catchError((e) {
      return null;
    });
    if (users == null) {
      sendMessage(
          SendMsg(cmd: CommandType.getUsers, usersId: [0]).toJSON());
    }
    return users;
  }

  void getUpdate() {
    sendMessage(SendMsg(cmd: CommandType.getUpdate).toJSON());
  }
}
