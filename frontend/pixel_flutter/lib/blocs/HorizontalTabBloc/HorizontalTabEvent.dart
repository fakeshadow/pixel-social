import 'package:equatable/equatable.dart';

class HorizontalTabEvent extends Equatable {
  HorizontalTabEvent([List props = const []]) : super(props);
}

class Tapped extends HorizontalTabEvent {
  final int index;
  Tapped({this.index}):super([index]);
}

