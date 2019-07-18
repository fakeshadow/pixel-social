import 'package:flutter/cupertino.dart';

class Talk {
  final int id, privacy, owner;
  final String name, description;
  final List<int> admin, users;

  Talk(
      {this.id,
      this.name,
      this.description,
      this.privacy,
      this.owner,
      this.admin,
      this.users});

  Map<String, dynamic> toMap() {
    final map = <String, dynamic>{
      'id': id,
      'name': name,
      'description': description,
      'privacy': privacy,
      'owner': owner,
      'admin': admin,
      'users': users,
    };
    return map;
  }
}

enum CommandType {
  getTalks,
  getUsers,
  getHistory,
  getRelation,
  getUpdate,
  sendPubMsg,
  sendPrvMsg,
}

class SendMsg {
  final int talkId;
  final int userId;
  final List<int> usersId;
  final String msg;
  final CommandType cmd;

  SendMsg(
      {this.talkId, this.userId, this.usersId, this.msg, @required this.cmd});

  String toJSON() {
    String str;
    switch (cmd) {
      case CommandType.getTalks:
        str = '/talks {"talk_id": $talkId}';
        break;
      case CommandType.getHistory:
        str = '/history {"history": $talkId}';
        break;
      case CommandType.getUpdate:
        str = '/update';
        break;
      case CommandType.getUsers:
        str = '/users {"user_id": $userId}';
        break;
      case CommandType.getRelation:
        str = '/relation {}';
        break;
      case CommandType.sendPrvMsg:
        str = '/msg {"user_id": $userId, "msg": "$msg"}';
        break;
      case CommandType.sendPubMsg:
        str = '/msg {"talk_id": $talkId, "msg": "$msg"}';
        break;
    }
    return str;
  }
}
