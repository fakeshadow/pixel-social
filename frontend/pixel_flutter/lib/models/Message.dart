import 'package:equatable/equatable.dart';

class PublicMessage extends Equatable {
  final int talkId;
  final DateTime dateTime;
  final String message;

  PublicMessage({this.talkId, this.dateTime, this.message})
      : super([talkId, dateTime, message]);
}

class PrivateMessage extends Equatable {
  final int userId;
  final DateTime dateTime;
  final String message;

  PrivateMessage({this.userId, this.dateTime, this.message})
      : super([userId, dateTime, message]);
}