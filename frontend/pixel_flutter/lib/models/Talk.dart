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
}

class GetTalks {
  final int talkId;

  GetTalks({this.talkId});

  String toJSON() => '/talks {"session_id": 0, "talk_id": $talkId}';
}

class GetUsers {
  final int talkId;

  GetUsers({this.talkId});

  String toJSON() => '/users {"session_id": 0, "talk_id": $talkId}';
}
