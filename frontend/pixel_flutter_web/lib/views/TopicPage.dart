import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter_web/components/BasicSliverPadding.dart';

import 'package:pixel_flutter_web/env.dart';

import 'package:pixel_flutter_web/blocs/TopicBlocs.dart';

import 'package:pixel_flutter_web/components/FloatingAppBar.dart';
import 'package:pixel_flutter_web/components/BasicLayout.dart';
import 'package:pixel_flutter_web/components/TopicTile.dart';
import 'package:pixel_flutter_web/components/PostsList.dart';
import 'package:pixel_flutter_web/components/SideMenu.dart';

import 'package:pixel_flutter_web/models/Topic.dart';

class TopicPage extends StatefulWidget with env {
  final Topic topic;

  TopicPage({Key key, @required this.topic}) : super(key: key);

  @override
  _TopicPageState createState() => _TopicPageState();
}

class _TopicPageState extends State<TopicPage> {
  TopicBloc _topicBloc;
  final _scrollController = ScrollController();
  final _scrollThreshold = 300.0;

  @override
  void initState() {
    _topicBloc = TopicBloc();
    _topicBloc.dispatch(GetTopic(topicId: widget.topic.id));
    _scrollController.addListener(_onScroll);
    super.initState();
  }

  @override
  void dispose() {
    _topicBloc.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return BasicLayout(
      scrollView: scrollView(_scrollController, widget.topic),
      sideMenu: SideMenu(),
    );
  }

  Widget scrollView(scrollController, Topic topic) {
    return Scrollbar(
      child: CustomScrollView(controller: scrollController, slivers: [
        FloatingAppBar(title: topic.title),
        BasicSliverPadding(
            sliver: TopicItem(
          topicBloc: _topicBloc,
        )),
        BasicSliverPadding(
            sliver: PostsList(
          topicBloc: _topicBloc,
        )),
      ]),
    );
  }

  void _onScroll() {
    final maxScroll = _scrollController.position.maxScrollExtent;
    final currentScroll = _scrollController.position.pixels;
    if (maxScroll - currentScroll <= _scrollThreshold) {
      _topicBloc.dispatch(GetTopic(topicId: widget.topic.id));
    }
  }
}

// ToDo: make a different container for topic item
class TopicItem extends StatelessWidget {
  final TopicBloc topicBloc;

  TopicItem({this.topicBloc});

  @override
  Widget build(BuildContext context) {
    return SliverToBoxAdapter(
        child: BlocBuilder(
      bloc: topicBloc,
      builder: (context, state) {
        if (state is TopicLoaded) {
          return TopicTile(topic: state.topic);
        } else {
          return Container();
        }
      },
    ));
  }
}
