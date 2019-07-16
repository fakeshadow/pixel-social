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

class GetTalks {
  final int talkId;

  GetTalks({this.talkId});

  String toJSON() => '/talks {"talk_id": $talkId}';
}

class GetUsers {
  final int talkId;

  GetUsers({this.talkId});

  String toJSON() => '/users {"talk_id": $talkId}';
}

class SendPubMsg {
  final int talkId;
  final String msg;

  SendPubMsg({this.talkId, this.msg});

  String toJSON() => '/msg {"talk_id": $talkId, "msg": "$msg"}';
}

class SendPrvMsg {
  final int userId;
  final String msg;

  SendPrvMsg({this.userId, this.msg});

  String toJSON() => '/msg {"user_id": $userId, "msg": "$msg"}';
}
