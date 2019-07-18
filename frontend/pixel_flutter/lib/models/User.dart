import 'package:equatable/equatable.dart';

class User extends Equatable {
  final int id, privilege;
  final String username, email, avatarUrl, signature, token;

  User(
      {this.id,
      this.privilege,
      this.username,
      this.email,
      this.avatarUrl,
      this.signature,
      this.token})
      : super([id, privilege, username, email, avatarUrl, signature, token]);

  Map<String, dynamic> toMap() {
    return <String, dynamic>{
      'id': id,
      'privilege': privilege,
      'username': username,
      'email': email,
      'avatarUrl': avatarUrl,
      'signature': signature
    };
  }
}
