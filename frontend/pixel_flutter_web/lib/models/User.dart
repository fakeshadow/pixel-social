import 'package:equatable/equatable.dart';

class User extends Equatable {
  final int id;
  final String username, email, avatarUrl, signature, token;

  User(
      {this.id,
      this.username,
      this.email,
      this.avatarUrl,
      this.signature,
      this.token})
      : super([id, username, email, avatarUrl, signature, token]);
}
