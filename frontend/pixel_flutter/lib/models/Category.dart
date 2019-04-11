import 'package:equatable/equatable.dart';

class Category extends Equatable {
  final int id;
  final String name, theme;

  Category({this.id, this.name, this.theme}) : super([id, name, theme]);
}
