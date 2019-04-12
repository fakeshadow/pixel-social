import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Category.dart';

abstract class CategoryState extends Equatable {
  CategoryState([List props = const []]) : super(props);
}

class CategoryLoading extends CategoryState {}

class CategoryLoaded extends CategoryState {
  final List<Category> categories;

  CategoryLoaded({this.categories}) : super([categories]);

  /// no need to use copy state for now but it's there for future use
//  CategoryLoaded copyWith(List<Category> categories) {
//    return CategoryLoaded(categories: categories ?? this.categories);
//  }
}

class CategoryFailed extends CategoryState {}
