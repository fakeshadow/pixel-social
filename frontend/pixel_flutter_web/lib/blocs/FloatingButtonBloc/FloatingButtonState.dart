import 'package:equatable/equatable.dart';
import 'package:flutter_web/widgets.dart';

abstract class FloatingButtonState extends Equatable {
  FloatingButtonState([List props = const []]) : super(props);
}

class IsVisible extends FloatingButtonState {
  final bool isVisible;

  IsVisible({@required this.isVisible}) : super([isVisible]);
}
