import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Category.dart';

abstract class CategoryState extends Equatable {
  CategoryState({List props = const[]}) : super(props);
}

class CategoryLoading extends CategoryState {}

class CategoryLoaded extends CategoryState {
    final List<Category> categories;

    CategoryLoaded([this.categories = const []]);
}

class CategoryFailed extends CategoryState {}