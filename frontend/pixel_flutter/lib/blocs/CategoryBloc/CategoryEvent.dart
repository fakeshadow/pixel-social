import 'package:equatable/equatable.dart';

abstract class CategoryEvent extends Equatable {
  CategoryEvent([List props = const []]) : super(props);
}

class LoadCategories extends CategoryEvent {}
