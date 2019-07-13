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

  Future<List<Talk>> init() async {
    sockets.connect();
    final talks = DataBase.getTalks();
    return talks;
  }

  void addListener(Function callback) {
    sockets.addListener(callback);
  }

  void sendMessage(String msg) {
    sockets.send(msg);
  }
}
