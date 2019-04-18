import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart' show BlocBuilder;

import 'package:pixel_flutter/blocs/TopicBlocs.dart';

import 'package:pixel_flutter/components/NavigationBar/SliverNavBar.dart';

import 'package:pixel_flutter/components/Loader/CenterLoader.dart';
import 'package:pixel_flutter/components/Loader/BottomLoader.dart';
import 'package:pixel_flutter/components/Topic/TopicList.dart';

import 'package:pixel_flutter/models/Category.dart';

class TopicsPage extends StatefulWidget {
  final Category category;

  TopicsPage({@required this.category});

  @override
  _TopicsPageState createState() => _TopicsPageState();
}

class _TopicsPageState extends State<TopicsPage> {
  int _categoryId;
  String _categoryName;
  String _categoryTheme;
  final _scrollController = ScrollController();
  final TopicBloc _topicBloc = TopicBloc();
  final _scrollThreshold = 200.0;

  @override
  void initState() {
    _categoryId = widget.category.id != null ? widget.category.id : 1;
    _categoryName =
        widget.category.name != null ? widget.category.name : 'PixelShare';
    _categoryTheme = widget.category.theme;
    _topicBloc.dispatch(GetTopics(categoryId: _categoryId));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
        bloc: _topicBloc,
        builder: (BuildContext context, TopicState state) {
          return Hero(
            tag: _categoryName,
            child: Scaffold(
              body: CustomScrollView(
                  controller: _scrollController,
                  slivers: <Widget>[
                    SliverNavBar(title: _categoryName, theme:_categoryTheme),
                    TopicList(state)
                  ]),
            ),
          );
        });
  }

  @override
  void dispose() {
    _topicBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(GetTopics(categoryId: _categoryId));
    }
  }
}

class TopicList extends StatelessWidget {
  final state;

  TopicList(this.state);

  @override
  Widget build(BuildContext context) {
    if (state is TopicError) {
      return CenterLoader();
    }
    if (state is TopicLoaded) {
      if (state.topics.isEmpty) {
        return CenterLoader();
      }
      return SliverList(
        delegate: SliverChildBuilderDelegate((context, index) {
          return index >= state.topics.length
              ? BottomLoader()
              : TopicView(state.topics[index]);
        },
            childCount: state.hasReachedMax
                ? state.topics.length
                : state.topics.length + 1),
      );
    } else {
      return CenterLoader();
    }
  }
}
